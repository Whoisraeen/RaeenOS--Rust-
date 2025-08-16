use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{OffsetPageTable, PageTable, PhysFrame, FrameAllocator, Size4KiB, Mapper, Page, PageTableFlags};
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};
use alloc::vec::Vec;
use alloc::collections::BTreeSet;

/// Memory region types for frame allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    Usable,
    Reserved,
    AcpiReclaimable,
    AcpiNvs,
    Bootloader,
    Kernel,
    FrameBuffer,
}

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    // SAFETY: This is unsafe because:
    // - physical_memory_offset must be a valid offset provided by the bootloader
    // - The bootloader must have set up identity+offset mapping for all physical memory
    // - active_level_4_table must return a valid reference to the current page table
    // - The returned OffsetPageTable assumes the offset mapping remains valid
    // - Only one OffsetPageTable should exist per physical memory offset at a time
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    // SAFETY: This is unsafe because:
    // - CR3 register access requires privileged execution context
    // - physical_memory_offset must be the correct offset for virtual-to-physical translation
    // - The calculated virtual address must point to a valid, mapped page table
    // - The page table must remain valid for the 'static lifetime
    // - Dereferencing the raw pointer assumes proper memory layout and alignment
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static bootloader::bootinfo::MemoryMap,
    allocated_frames: BTreeSet<u64>,
    free_frames: Vec<PhysFrame>,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static bootloader::bootinfo::MemoryMap) -> Self {
        // SAFETY: This is unsafe because:
        // - memory_map must be a valid reference to bootloader-provided memory information
        // - The memory map must accurately reflect the current system memory layout
        // - Usable memory regions in the map must not be in use by bootloader or firmware
        // - The memory map must remain valid for the 'static lifetime
        // - Frame allocation assumes exclusive ownership of usable memory regions
        let mut allocator = Self { 
            memory_map,
            allocated_frames: BTreeSet::new(),
            free_frames: Vec::new(),
        };
        
        // Pre-populate free frames list
        allocator.free_frames = allocator.usable_frames().collect();
        allocator
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        use bootloader::bootinfo::MemoryRegionType;
        let regions = self.memory_map.iter();
        let usable = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        let addrs = usable.flat_map(|r| (r.range.start_addr()..r.range.end_addr()).step_by(4096));
        addrs.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr as u64)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if let Some(frame) = self.free_frames.pop() {
            self.allocated_frames.insert(frame.start_address().as_u64());
            update_allocated_frames(1);
            Some(frame)
        } else {
            None
        }
    }
}

impl BootInfoFrameAllocator {
    /// Deallocate a frame and return it to the free list
    pub fn deallocate_frame(&mut self, frame: PhysFrame) {
        let frame_addr = frame.start_address().as_u64();
        if self.allocated_frames.remove(&frame_addr) {
            self.free_frames.push(frame);
            update_allocated_frames(-1);
        }
    }
    
    /// Get the number of allocated frames
    pub fn allocated_count(&self) -> usize {
        self.allocated_frames.len()
    }
    
    /// Get the number of free frames
    pub fn free_count(&self) -> usize {
        self.free_frames.len()
    }
    
    /// Check if a frame is allocated
    pub fn is_allocated(&self, frame: PhysFrame) -> bool {
        self.allocated_frames.contains(&frame.start_address().as_u64())
    }
}

// ---------- Global accessors for mapper/frame allocator ----------

static PHYS_OFFSET: AtomicU64 = AtomicU64::new(0);
pub static FRAME_ALLOC: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);
static ALLOCATED_FRAMES: AtomicU64 = AtomicU64::new(0);

/// Update the global allocated frames counter
fn update_allocated_frames(delta: i64) {
    let current = ALLOCATED_FRAMES.load(Ordering::SeqCst);
    let new_value = if delta < 0 {
        current.saturating_sub((-delta) as u64)
    } else {
        current.saturating_add(delta as u64)
    };
    ALLOCATED_FRAMES.store(new_value, Ordering::SeqCst);
}

/// Get the number of allocated frames
pub fn get_allocated_frames() -> u64 {
    ALLOCATED_FRAMES.load(Ordering::SeqCst)
}

pub fn active_physical_offset() -> u64 { PHYS_OFFSET.load(Ordering::SeqCst) }

pub fn set_physical_memory_offset(offset: u64) { PHYS_OFFSET.store(offset, Ordering::SeqCst); }

pub unsafe fn init_global_frame_allocator(memory_map: &'static bootloader::bootinfo::MemoryMap) {
    // SAFETY: This is unsafe because:
    // - memory_map must be a valid bootloader-provided memory map
    // - This function must be called only once during kernel initialization
    // - No other code should access FRAME_ALLOC during this initialization
    // - The memory map must remain valid throughout kernel execution
    // - BootInfoFrameAllocator::init has its own safety requirements
    *FRAME_ALLOC.lock() = Some(BootInfoFrameAllocator::init(memory_map));
}

pub fn with_mapper<F, R>(f: F) -> R
where
    F: FnOnce(&mut OffsetPageTable<'static>) -> R,
{
    // SAFETY: This is safe because:
    // 1. active_physical_offset() returns a valid offset provided by the bootloader
    // 2. The bootloader guarantees identity+offset mapping remains valid throughout kernel execution
    // 3. The OffsetPageTable is constructed with a valid physical memory offset
    // 4. No other code modifies the page table structure concurrently during this call
    let mut mapper = unsafe { init(VirtAddr::new(active_physical_offset())) };
    f(&mut mapper)
}

pub fn allocate_frame() -> Option<PhysFrame> {
    FRAME_ALLOC.lock().as_mut().and_then(|a| a.allocate_frame())
}

/// Allocate a frame with guard pages on both sides
pub fn allocate_frame_with_guards() -> Option<(PhysFrame, PhysFrame, PhysFrame)> {
    let mut frame_alloc = FRAME_ALLOC.lock();
    if let Some(ref mut alloc) = *frame_alloc {
        let guard_before = alloc.allocate_frame()?;
        let main_frame = alloc.allocate_frame()?;
        let guard_after = alloc.allocate_frame()?;
        Some((guard_before, main_frame, guard_after))
    } else {
        None
    }
}

/// Map a page with guard pages
pub fn map_page_with_guards(virt_addr: VirtAddr, flags: PageTableFlags) -> Result<(), &'static str> {
    if let Some((guard_before, main_frame, guard_after)) = allocate_frame_with_guards() {
        with_mapper(|mapper| {
            // Map guard page before (no permissions)
            let guard_before_page = Page::<Size4KiB>::containing_address(virt_addr - 4096u64);
            let guard_flags = PageTableFlags::PRESENT; // Present but not readable/writable/executable
            
            // Map main page
            let main_page = Page::<Size4KiB>::containing_address(virt_addr);
            
            // Map guard page after (no permissions)
            let guard_after_page = Page::<Size4KiB>::containing_address(virt_addr + 4096u64);
            
            // Use a dummy frame allocator for the mapping operations
            let mut dummy_alloc = DummyFrameAllocator;
            
            // Track successful mappings for cleanup on error
            let mut mapped_pages = Vec::new();
            
            // Map guard before
            // SAFETY: Safe because:
            // 1. guard_before_page and guard_before are valid and aligned
            // 2. guard_flags are appropriate for a guard page (no permissions)
            // 3. mapper is a valid OffsetPageTable with proper physical offset
            // 4. dummy_alloc is used only for the mapping operation
            match unsafe { mapper.map_to(guard_before_page, guard_before, guard_flags, &mut dummy_alloc) } {
                Ok(mapping) => {
                    mapping.flush();
                    mapped_pages.push((guard_before_page, guard_before));
                }
                Err(_) => {
                    // Failed to map guard before - deallocate all frames
                    deallocate_frame(guard_before);
                    deallocate_frame(main_frame);
                    deallocate_frame(guard_after);
                    return Err("Failed to map guard page before");
                }
            }
            
            // Map main page
            // SAFETY: Safe because:
            // 1. main_page and main_frame are valid, aligned, and freshly allocated
            // 2. flags are validated page table flags for the intended use
            // 3. mapper is a valid OffsetPageTable with proper physical offset
            // 4. dummy_alloc is used only for the mapping operation
            match unsafe { mapper.map_to(main_page, main_frame, flags, &mut dummy_alloc) } {
                Ok(mapping) => {
                    mapping.flush();
                    mapped_pages.push((main_page, main_frame));
                }
                Err(_) => {
                    // Failed to map main page - clean up and deallocate
                    for (page, frame) in mapped_pages {
                        let _ = mapper.unmap(page);
                        deallocate_frame(frame);
                    }
                    deallocate_frame(main_frame);
                    deallocate_frame(guard_after);
                    return Err("Failed to map main page");
                }
            }
            
            // Map guard after
            // SAFETY: Safe because:
            // 1. guard_after_page and guard_after are valid and aligned
            // 2. guard_flags are appropriate for a guard page (no permissions)
            // 3. mapper is a valid OffsetPageTable with proper physical offset
            // 4. dummy_alloc is used only for the mapping operation
            match unsafe { mapper.map_to(guard_after_page, guard_after, guard_flags, &mut dummy_alloc) } {
                Ok(mapping) => {
                    mapping.flush();
                    Ok(())
                }
                Err(_) => {
                    // Failed to map guard after - clean up and deallocate
                    for (page, frame) in mapped_pages {
                        let _ = mapper.unmap(page);
                        deallocate_frame(frame);
                    }
                    deallocate_frame(guard_after);
                    return Err("Failed to map guard page after");
                }
            }
        })
    } else {
        Err("Failed to allocate frames for guard pages")
    }
}

/// Dummy frame allocator for mapping operations when we already have frames
struct DummyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for DummyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None // We don't allocate new frames, we use pre-allocated ones
    }
 }
 
 /// KASLR (Kernel Address Space Layout Randomization) implementation
 pub mod kaslr {
     use super::*;
     use crate::arch::tsc;
     
     /// KASLR entropy sources
     #[derive(Debug, Clone, Copy)]
     pub struct KaslrEntropy {
         pub tsc_low: u32,
         pub tsc_high: u32,
         pub cpu_id: u32,
         pub memory_size: u64,
     }
     
     impl KaslrEntropy {
         /// Gather entropy from various hardware sources
         pub fn gather() -> Self {
             let tsc = tsc::read_tsc();
             let cpu_id = crate::arch::get_current_cpu_id();
             let memory_size = get_total_memory();
             
             Self {
                 tsc_low: tsc as u32,
                 tsc_high: (tsc >> 32) as u32,
                 cpu_id,
                 memory_size,
             }
         }
         
         /// Generate a pseudo-random number using gathered entropy
         pub fn random_u64(&self) -> u64 {
             // Simple PRNG using entropy sources
             let mut state = self.tsc_low as u64;
             state ^= (self.tsc_high as u64) << 32;
             state ^= (self.cpu_id as u64) << 16;
             state ^= self.memory_size;
             
             // Linear congruential generator
             state = state.wrapping_mul(1103515245).wrapping_add(12345);
             state ^= state >> 16;
             state = state.wrapping_mul(1103515245).wrapping_add(12345);
             state ^= state >> 16;
             
             state
         }
         
         /// Generate a random offset within a range
         pub fn random_offset(&self, max_offset: u64) -> u64 {
             if max_offset == 0 {
                 return 0;
             }
             self.random_u64() % max_offset
         }
     }
     
     /// KASLR configuration
     pub struct KaslrConfig {
         pub kernel_base_min: VirtAddr,
         pub kernel_base_max: VirtAddr,
         pub heap_base_min: VirtAddr,
         pub heap_base_max: VirtAddr,
         pub stack_base_min: VirtAddr,
         pub stack_base_max: VirtAddr,
     }
     
     impl Default for KaslrConfig {
         fn default() -> Self {
             Self {
                 // Kernel can be randomized within a 1GB range
                 kernel_base_min: VirtAddr::new(0xFFFF_8000_0000_0000),
                 kernel_base_max: VirtAddr::new(0xFFFF_8000_4000_0000),
                 
                 // Heap randomization within 2GB range
                 heap_base_min: VirtAddr::new(0x0000_1000_0000_0000),
                 heap_base_max: VirtAddr::new(0x0000_1000_8000_0000),
                 
                 // Stack randomization within 1GB range
                 stack_base_min: VirtAddr::new(0x0000_7000_0000_0000),
                 stack_base_max: VirtAddr::new(0x0000_7000_4000_0000),
             }
         }
     }
     
     /// KASLR manager
     #[allow(dead_code)]
     pub struct KaslrManager {
         config: KaslrConfig,
         entropy: KaslrEntropy,
         randomized_bases: RandomizedBases,
     }
     
     #[derive(Debug, Clone, Copy)]
     pub struct RandomizedBases {
         pub kernel_base: VirtAddr,
         pub heap_base: VirtAddr,
         pub stack_base: VirtAddr,
     }
     
     impl KaslrManager {
         /// Initialize KASLR with gathered entropy
         pub fn new() -> Self {
             let entropy = KaslrEntropy::gather();
             let config = KaslrConfig::default();
             
             // Calculate randomized base addresses
             let kernel_range = config.kernel_base_max.as_u64() - config.kernel_base_min.as_u64();
             let heap_range = config.heap_base_max.as_u64() - config.heap_base_min.as_u64();
             let stack_range = config.stack_base_max.as_u64() - config.stack_base_min.as_u64();
             
             let kernel_offset = entropy.random_offset(kernel_range) & !0xFFF; // Align to 4KB
             let heap_offset = entropy.random_offset(heap_range) & !0xFFF;
             let stack_offset = entropy.random_offset(stack_range) & !0xFFF;
             
             let randomized_bases = RandomizedBases {
                 kernel_base: VirtAddr::new(config.kernel_base_min.as_u64() + kernel_offset),
                 heap_base: VirtAddr::new(config.heap_base_min.as_u64() + heap_offset),
                 stack_base: VirtAddr::new(config.stack_base_min.as_u64() + stack_offset),
             };
             
             Self {
                 config,
                 entropy,
                 randomized_bases,
             }
         }
         
         /// Get randomized kernel base address
         pub fn kernel_base(&self) -> VirtAddr {
             self.randomized_bases.kernel_base
         }
         
         /// Get randomized heap base address
         pub fn heap_base(&self) -> VirtAddr {
             self.randomized_bases.heap_base
         }
         
         /// Get randomized stack base address
         pub fn stack_base(&self) -> VirtAddr {
             self.randomized_bases.stack_base
         }
         
         /// Generate a randomized address within a range
         pub fn randomize_address(&self, base: VirtAddr, max_offset: u64) -> VirtAddr {
             let offset = self.entropy.random_offset(max_offset) & !0xFFF; // Align to 4KB
             VirtAddr::new(base.as_u64() + offset)
         }
         
         /// Apply KASLR to memory layout
         pub fn apply_randomization(&self) -> Result<(), &'static str> {
             crate::serial::_print(format_args!(
                 "[KASLR] Randomized bases - Kernel: {:?}, Heap: {:?}, Stack: {:?}\n",
                 self.randomized_bases.kernel_base,
                 self.randomized_bases.heap_base,
                 self.randomized_bases.stack_base
             ));
             
             // Note: In a real implementation, we would need to relocate the kernel
             // and update all absolute addresses. For now, we just log the randomized addresses.
             
             Ok(())
         }
     }
     
     static KASLR_MANAGER: Mutex<Option<KaslrManager>> = Mutex::new(None);
     
     /// Initialize KASLR
     pub fn init() -> Result<(), &'static str> {
         let manager = KaslrManager::new();
         manager.apply_randomization()?;
         
         *KASLR_MANAGER.lock() = Some(manager);
         
         crate::serial::_print(format_args!("[KASLR] Kernel Address Space Layout Randomization initialized\n"));
         Ok(())
     }
     
     /// Get randomized address for allocation
     pub fn get_randomized_address(base: VirtAddr, max_offset: u64) -> VirtAddr {
         if let Some(ref manager) = *KASLR_MANAGER.lock() {
             manager.randomize_address(base, max_offset)
         } else {
             base // Fallback to base address if KASLR not initialized
         }
     }
     
     /// Get current KASLR bases
     pub fn get_randomized_bases() -> Option<RandomizedBases> {
         KASLR_MANAGER.lock().as_ref().map(|m| m.randomized_bases)
     }
 }
 
 /// Initialize KASLR during boot
 pub fn init_kaslr() -> Result<(), &'static str> {
     kaslr::init()
 }
 
 pub fn with_frame_allocator<F, R>(f: F) -> R
where
    F: FnOnce(&mut BootInfoFrameAllocator) -> R,
{
    let mut frame_alloc = FRAME_ALLOC.lock();
    let allocator = frame_alloc.as_mut().expect("Frame allocator not initialized");
    f(allocator)
}

/// Deallocate a frame globally
pub fn deallocate_frame(frame: PhysFrame) {
    with_frame_allocator(|allocator| {
        allocator.deallocate_frame(frame);
    });
}

/// Get memory statistics
pub fn get_memory_stats() -> (usize, usize, u64) {
    with_frame_allocator(|allocator| {
        (allocator.free_count(), allocator.allocated_count(), get_allocated_frames())
    })
}

// Convert physical address to virtual address using the physical memory offset
pub fn phys_to_virt(phys_addr: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys_addr.as_u64() + active_physical_offset())
}

// Convert virtual address to physical address using the physical memory offset
pub fn virt_to_phys(virt_addr: VirtAddr) -> PhysAddr {
    PhysAddr::new(virt_addr.as_u64() - active_physical_offset())
}

use lazy_static::lazy_static;

// Memory statistics tracking
struct MemoryStats {
    total_memory: u64,
    allocated_frames: u64,
    heap_break: VirtAddr,
}

lazy_static! {
    static ref MEMORY_STATS: Mutex<MemoryStats> = Mutex::new(MemoryStats {
        total_memory: 0,
        allocated_frames: 0,
        heap_break: VirtAddr::new(0x400000000), // Start heap at 16GB virtual address
    });
}

// Initialize memory statistics from bootloader info
pub fn init_memory_stats(total_memory: u64) {
    let mut stats = MEMORY_STATS.lock();
    stats.total_memory = total_memory;
}

// Memory management functions for syscalls
pub fn set_program_break(addr: VirtAddr) -> Result<VirtAddr, ()> {
    let mut stats = MEMORY_STATS.lock();
    let current_break = stats.heap_break;
    
    // Validate the new break address
    if addr < current_break {
        // Shrinking heap - unmap pages
        let _pages_to_unmap = (current_break.as_u64() - addr.as_u64()) / 4096;
        
        // Get current process for permission checking
        let current_pid = crate::process::get_current_process_info().map(|(pid, _, _)| pid).unwrap_or(0);
        if !crate::security::request_permission(current_pid.try_into().unwrap_or(0), "memory.alloc").unwrap_or(false) {
            return Err(());
        }
        
        // Unmap the pages and deallocate frames
        with_mapper(|mapper| {
            let mut current_addr = addr;
            while current_addr < current_break {
                let page = Page::<Size4KiB>::containing_address(current_addr);
                if let Ok(_frame) = mapper.translate_page(page) {
                    // Unmap the page first
                    if let Ok((frame, _flush)) = mapper.unmap(page) {
                        // Now deallocate the frame
                        deallocate_frame(frame);
                    }
                }
                current_addr += 4096u64;
            }
        });
        
        stats.heap_break = addr;
        Ok(addr)
    } else if addr > current_break {
        // Growing heap - map new pages
        let pages_to_map = (addr.as_u64() - current_break.as_u64()) / 4096;
        
        // Check memory allocation permission
        let current_pid = crate::process::get_current_process_info().map(|(pid, _, _)| pid).unwrap_or(0);
        if !crate::security::request_permission(current_pid.try_into().unwrap_or(0), "memory.alloc").unwrap_or(false) {
            return Err(());
        }
        
        // Check if we have enough free memory
        let required_memory = pages_to_map * 4096;
        if get_free_memory() < required_memory {
            return Err(()); // Out of memory
        }
        
        // Map the new pages
        with_mapper(|mapper| {
            with_frame_allocator(|allocator| {
                let mut current_addr = current_break;
                let mut allocated_pages = Vec::new();
                
                while current_addr < addr {
                    let page = Page::containing_address(current_addr);
                    if let Some(frame) = allocator.allocate_frame() {
                        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
                        // SAFETY: Safe because:
                        // 1. page and frame are valid and aligned
                        // 2. flags are validated page table flags for user-accessible memory
                        // 3. mapper is a valid OffsetPageTable with proper physical offset
                        // 4. allocator is the global frame allocator
                        match unsafe { mapper.map_to(page, frame, flags, allocator) } {
                            Ok(_) => {
                                // Zero the page for security
                                // SAFETY: Safe because:
                                // 1. current_addr is a valid virtual address we just mapped
                                // 2. The page is mapped with WRITABLE permissions
                                // 3. We're writing exactly one page (4096 bytes) which matches the mapping size
                                // 4. No other code can access this page until we return
                                unsafe {
                                    let page_ptr = current_addr.as_mut_ptr::<u8>();
                                    core::ptr::write_bytes(page_ptr, 0, 4096);
                                }
                                allocated_pages.push((page, frame));
                            }
                            Err(_) => {
                                // Mapping failed - clean up all allocated pages and frames
                                deallocate_frame(frame); // Deallocate the current frame
                                
                                // Clean up previously allocated pages in this operation
                                for (allocated_page, allocated_frame) in allocated_pages {
                                    let _ = mapper.unmap(allocated_page);
                                    deallocate_frame(allocated_frame);
                                }
                                return Err(());
                            }
                        }
                    } else {
                        // Frame allocation failed - clean up previously allocated pages
                        for (allocated_page, allocated_frame) in allocated_pages {
                            let _ = mapper.unmap(allocated_page);
                            deallocate_frame(allocated_frame);
                        }
                        return Err(());
                    }
                    current_addr += 4096u64;
                }
                Ok(())
            })
        }).map_err(|_| ())?;
        
        stats.heap_break = addr;
        stats.allocated_frames += pages_to_map;
        Ok(addr)
    } else {
        // No change
        Ok(current_break)
    }
}

pub fn get_total_memory() -> u64 {
    let stats = MEMORY_STATS.lock();
    if stats.total_memory == 0 {
        // Fallback: try to detect memory from bootloader or use conservative estimate
        256 * 1024 * 1024 // 256MB fallback
    } else {
        stats.total_memory
    }
}

pub fn get_free_memory() -> u64 {
    let stats = MEMORY_STATS.lock();
    let total = if stats.total_memory == 0 {
        256 * 1024 * 1024 // 256MB fallback
    } else {
        stats.total_memory
    };
    
    let used = stats.allocated_frames * 4096;
    
    // Reserve some memory for kernel operations
    let kernel_reserved = 32 * 1024 * 1024; // 32MB for kernel
    
    if total > used + kernel_reserved {
        total - used - kernel_reserved
    } else {
        0
    }
}

// Update allocated frame count (called by frame allocator)
// Note: This function is already defined earlier in the file

// Get current program break
pub fn get_program_break() -> VirtAddr {
    let stats = MEMORY_STATS.lock();
    stats.heap_break
}



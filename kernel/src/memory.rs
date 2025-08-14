use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{OffsetPageTable, PageTable, PhysFrame, FrameAllocator, Size4KiB, Mapper, Page, PageTableFlags};
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;
    let (level_4_table_frame, _) = Cr3::read();
    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
    &mut *page_table_ptr
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static bootloader::bootinfo::MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static bootloader::bootinfo::MemoryMap) -> Self {
        Self { memory_map, next: 0 }
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
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

// ---------- Global accessors for mapper/frame allocator ----------

static PHYS_OFFSET: AtomicU64 = AtomicU64::new(0);
pub static FRAME_ALLOC: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);

pub fn active_physical_offset() -> u64 { PHYS_OFFSET.load(Ordering::SeqCst) }

pub fn set_physical_memory_offset(offset: u64) { PHYS_OFFSET.store(offset, Ordering::SeqCst); }

pub unsafe fn init_global_frame_allocator(memory_map: &'static bootloader::bootinfo::MemoryMap) {
    *FRAME_ALLOC.lock() = Some(BootInfoFrameAllocator::init(memory_map));
}

pub fn with_mapper<F, R>(f: F) -> R
where
    F: FnOnce(&mut OffsetPageTable<'static>) -> R,
{
    // Safe because we rely on bootloader-provided identity+offset mapping that stays valid
    let mut mapper = unsafe { init(VirtAddr::new(active_physical_offset())) };
    f(&mut mapper)
}

pub fn allocate_frame() -> Option<PhysFrame> {
    FRAME_ALLOC.lock().as_mut().and_then(|a| a.allocate_frame())
}

pub fn with_frame_allocator<F, R>(f: F) -> R
where
    F: FnOnce(&mut BootInfoFrameAllocator) -> R,
{
    let mut frame_alloc = FRAME_ALLOC.lock();
    let allocator = frame_alloc.as_mut().expect("Frame allocator not initialized");
    f(allocator)
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
        let pages_to_unmap = (current_break.as_u64() - addr.as_u64()) / 4096;
        
        // Get current process for permission checking
        let current_pid = crate::process::get_current_process_info().map(|(pid, _, _)| pid).unwrap_or(0);
        if !crate::security::request_permission(current_pid.try_into().unwrap_or(0), "memory.alloc").unwrap_or(false) {
            return Err(());
        }
        
        // Unmap the pages (simplified)
        with_mapper(|mapper| {
            let mut current_addr = addr;
            while current_addr < current_break {
                if let Ok(_frame) = mapper.translate_page(Page::<Size4KiB>::containing_address(current_addr)) {
                    let _ = mapper.unmap(Page::<Size4KiB>::containing_address(current_addr));
                    // TODO: Implement frame deallocation
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
                while current_addr < addr {
                    let page = Page::containing_address(current_addr);
                    if let Some(frame) = allocator.allocate_frame() {
                        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
                        if unsafe { mapper.map_to(page, frame, flags, allocator) }.is_ok() {
                            // Zero the page for security
                            unsafe {
                                let page_ptr = current_addr.as_mut_ptr::<u8>();
                                core::ptr::write_bytes(page_ptr, 0, 4096);
                            }
                        } else {
                            // Mapping failed
                            // TODO: Implement frame deallocation
                            return Err(());
                        }
                    } else {
                        return Err(()); // Frame allocation failed
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
pub fn update_allocated_frames(delta: i64) {
    let mut stats = MEMORY_STATS.lock();
    if delta < 0 {
        let decrease = (-delta) as u64;
        stats.allocated_frames = stats.allocated_frames.saturating_sub(decrease);
    } else {
        stats.allocated_frames += delta as u64;
    }
}

// Get current program break
pub fn get_program_break() -> VirtAddr {
    let stats = MEMORY_STATS.lock();
    stats.heap_break
}



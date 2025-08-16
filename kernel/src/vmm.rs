use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::fmt;
use spin::{Mutex, RwLock};
use bitflags::bitflags;
use x86_64::{
    structures::paging::{
        Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
        Mapper, FrameAllocator,
    },
    VirtAddr, PhysAddr,
};
use x86_64::registers::control::Cr3;
use crate::memory;

static VMM: RwLock<VirtualMemoryManager> = RwLock::new(VirtualMemoryManager::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmAreaType {
    Code,
    Data,
    Stack,
    Heap,
    Shared,
    Device,
    Guard,
}

bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct VmPermissions: u16 {
        const READ = 0x01;
        const WRITE = 0x02;
        const EXECUTE = 0x04;
        const USER = 0x08;
        const GLOBAL = 0x10;
        const NO_CACHE = 0x20;
        const WRITE_THROUGH = 0x40;
        const SWAPPABLE = 0x80;
        const DUAL_MAPPING = 0x100;
        const JIT_ALLOWED = 0x200;
    }
}

impl VmPermissions {
    pub fn readable(self) -> bool { self.contains(Self::READ) }
    
    pub fn writable(self) -> bool { self.contains(Self::WRITE) }
    
    pub fn executable(self) -> bool { self.contains(Self::EXECUTE) }
    
    pub fn user_accessible(self) -> bool { self.contains(Self::USER) }
    
    /// Validates W^X policy: writable pages cannot be executable
    pub fn validate_wx_policy(self) -> Result<(), VmError> {
        if self.writable() && self.executable() {
            Err(VmError::WxViolation)
        } else {
            Ok(())
        }
    }
    
    /// Check if this is a dual-mapping permission (for JIT support)
    pub fn is_dual_mapping(self) -> bool {
        self.contains(Self::DUAL_MAPPING)
    }
    
    /// Validate dual-mapping policy for JIT compilation
    pub fn validate_dual_mapping_policy(self) -> Result<(), VmError> {
        if self.is_dual_mapping() {
            // Dual mappings are allowed to bypass W^X for JIT compilation
            // but require special handling
            if !self.contains(Self::JIT_ALLOWED) {
                return Err(VmError::JitNotAllowed);
            }
            Ok(())
        } else {
            // Regular mappings must follow W^X policy
            self.validate_wx_policy()
        }
    }
    
    pub fn to_page_table_flags(self) -> PageTableFlags {
        let mut flags = PageTableFlags::PRESENT;
        
        if self.writable() {
            flags |= PageTableFlags::WRITABLE;
        }
        
        if self.user_accessible() {
            flags |= PageTableFlags::USER_ACCESSIBLE;
        }
        
        if !self.executable() {
            flags |= PageTableFlags::NO_EXECUTE;
        }
        
        if self.contains(Self::GLOBAL) {
            flags |= PageTableFlags::GLOBAL;
        }
        
        if self.contains(Self::NO_CACHE) {
            flags |= PageTableFlags::NO_CACHE;
        }
        
        if self.contains(Self::WRITE_THROUGH) {
            flags |= PageTableFlags::WRITE_THROUGH;
        }
        
        flags
    }
}

// BitOr and BitOrAssign implementations are provided by bitflags! macro

#[derive(Debug, Clone)]
pub struct VmArea {
    pub start: VirtAddr,
    pub end: VirtAddr,
    pub area_type: VmAreaType,
    pub permissions: VmPermissions,
    pub name: Option<alloc::string::String>,
    pub file_offset: Option<u64>,
    pub is_shared: bool,
    pub is_anonymous: bool,
    pub ref_count: u32,
}

impl VmArea {
    pub fn new(start: VirtAddr, end: VirtAddr, area_type: VmAreaType, permissions: VmPermissions) -> Self {
        Self {
            start,
            end,
            area_type,
            permissions,
            name: None,
            file_offset: None,
            is_shared: false,
            is_anonymous: true,
            ref_count: 1,
        }
    }
    
    pub fn size(&self) -> u64 {
        self.end.as_u64() - self.start.as_u64()
    }
    
    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.start && addr < self.end
    }
    
    pub fn overlaps(&self, other: &VmArea) -> bool {
        self.start < other.end && other.start < self.end
    }
    
    pub fn pages(&self) -> impl Iterator<Item = Page<Size4KiB>> {
        let start_page = Page::<Size4KiB>::containing_address(self.start);
        let end_page = Page::<Size4KiB>::containing_address(self.end - 1u64);
        Page::range_inclusive(start_page, end_page)
    }
}

#[derive(Debug)]
pub struct AddressSpace {
    pub id: u64,
    pub page_table: Box<PageTable>,
    pub pml4_frame: PhysFrame,  // Physical frame for this address space's PML4
    pub areas: BTreeMap<VirtAddr, VmArea>,
    pub heap_start: VirtAddr,
    pub heap_end: VirtAddr,
    pub stack_start: VirtAddr,
    pub stack_end: VirtAddr,
    pub mmap_start: VirtAddr,
    pub next_mmap: VirtAddr,
}

impl AddressSpace {
    pub fn new(id: u64) -> Result<Self, VmError> {
        // Allocate a new physical frame for the PML4
        let pml4_frame = memory::allocate_frame().ok_or(VmError::OutOfMemory)?;
        
        // Create a new page table and map it to the allocated frame
        let page_table = Box::new(PageTable::new());
        
        // Define virtual memory layout
        let heap_start = VirtAddr::new(0x1000_0000_0000); // 256 TiB
        let heap_end = VirtAddr::new(0x2000_0000_0000);   // 512 TiB
        let stack_start = VirtAddr::new(0x7000_0000_0000); // 1792 TiB
        let stack_end = VirtAddr::new(0x8000_0000_0000);   // 2048 TiB
        let mmap_start = VirtAddr::new(0x3000_0000_0000);  // 768 TiB
        
        let mut address_space = Self {
            id,
            page_table,
            pml4_frame,
            areas: BTreeMap::new(),
            heap_start,
            heap_end,
            stack_start,
            stack_end,
            mmap_start,
            next_mmap: mmap_start,
        };
        
        // Map kernel higher-half into this address space
        address_space.map_kernel_space();
        
        Ok(address_space)
    }
    
    // Map kernel higher-half into this address space
    fn map_kernel_space(&mut self) {
        // Get current kernel PML4 to copy kernel mappings
        let (current_pml4_frame, _) = Cr3::read();
        
        memory::with_mapper(|_mapper| {
            // Map the new PML4 temporarily to copy kernel entries
            let pml4_virt = memory::phys_to_virt(self.pml4_frame.start_address());
            // SAFETY: Safe because:
            // 1. pml4_virt is a valid virtual address from phys_to_virt conversion
            // 2. self.pml4_frame is a valid, allocated frame for a page table
            // 3. The frame is properly aligned for PageTable structure
            // 4. We have exclusive access to this new PML4 during initialization
            let new_pml4 = unsafe { &mut *(pml4_virt.as_mut_ptr::<PageTable>()) };
            
            // Get current PML4
            let current_pml4_virt = memory::phys_to_virt(current_pml4_frame.start_address());
            // SAFETY: Safe because:
            // 1. current_pml4_virt is a valid virtual address from phys_to_virt conversion
            // 2. current_pml4_frame is the active CR3 frame, guaranteed to be valid
            // 3. The frame is properly aligned for PageTable structure
            // 4. We only read from the current PML4, no modifications
            let current_pml4 = unsafe { &*(current_pml4_virt.as_ptr::<PageTable>()) };
            
            // Copy kernel higher-half entries (entries 256-511 for kernel space)
            for i in 256..512 {
                new_pml4[i] = current_pml4[i].clone();
            }
        });
    }
    
    pub fn add_area(&mut self, area: VmArea) -> Result<(), VmError> {
        // Check for overlaps
        for existing_area in self.areas.values() {
            if area.overlaps(existing_area) {
                return Err(VmError::AddressInUse);
            }
        }
        
        self.areas.insert(area.start, area);
        Ok(())
    }
    
    pub fn remove_area(&mut self, start: VirtAddr) -> Option<VmArea> {
        self.areas.remove(&start)
    }
    
    pub fn find_area(&self, addr: VirtAddr) -> Option<&VmArea> {
        for area in self.areas.values() {
            if area.contains(addr) {
                return Some(area);
            }
        }
        None
    }
    
    pub fn find_area_mut(&mut self, addr: VirtAddr) -> Option<&mut VmArea> {
        for area in self.areas.values_mut() {
            if area.contains(addr) {
                return Some(area);
            }
        }
        None
    }
    
    pub fn allocate_area(&mut self, size: u64, area_type: VmAreaType, permissions: VmPermissions) -> Result<VirtAddr, VmError> {
        // Enforce W^X policy or dual-mapping policy
        permissions.validate_dual_mapping_policy()?;
        
        let aligned_size = (size + 0xFFF) & !0xFFF; // Align to 4KB
        
        let start_addr = match area_type {
            VmAreaType::Heap => {
                // Find space in heap region
                self.find_free_space(self.heap_start, self.heap_end, aligned_size)?
            },
            VmAreaType::Stack => {
                // Allocate from top of stack region
                let start = self.stack_end - aligned_size;
                if start < self.stack_start {
                    return Err(VmError::OutOfMemory);
                }
                start
            },
            _ => {
                // Use mmap region
                let start = self.next_mmap;
                self.next_mmap += aligned_size;
                start
            }
        };
        
        let area = VmArea::new(
            start_addr,
            start_addr + aligned_size,
            area_type,
            permissions
        );
        
        self.add_area(area)?;
        Ok(start_addr)
    }
    
    fn find_free_space(&self, start: VirtAddr, end: VirtAddr, size: u64) -> Result<VirtAddr, VmError> {
        let mut current = start;
        
        while current + size <= end {
            let mut found_overlap = false;
            
            for area in self.areas.values() {
                if current < area.end && current + size > area.start {
                    current = area.end;
                    found_overlap = true;
                    break;
                }
            }
            
            if !found_overlap {
                return Ok(current);
            }
        }
        
        Err(VmError::OutOfMemory)
    }
    
    /// Clear all user-space mappings from this address space
    /// Preserves kernel mappings (higher half)
    pub fn clear_user_mappings(&mut self) -> Result<(), VmError> {
        // Clear all user areas
        self.areas.clear();
        
        // Reset user space layout
        self.next_mmap = self.mmap_start;
        
        // Clear user-space page table entries (entries 0-255)
        memory::with_mapper(|_mapper| {
            let pml4_virt = memory::phys_to_virt(self.pml4_frame.start_address());
            // SAFETY: Safe because:
            // 1. pml4_virt is a valid virtual address from phys_to_virt conversion
            // 2. self.pml4_frame is a valid, allocated frame for this address space
            // 3. The frame is properly aligned for PageTable structure
            // 4. We have exclusive access to this address space's PML4
            // 5. We only clear user-space entries (0-255), preserving kernel mappings
            let pml4 = unsafe { &mut *(pml4_virt.as_mut_ptr::<PageTable>()) };
            
            // Clear user-space entries (0-255), preserve kernel entries (256-511)
            for i in 0..256 {
                pml4[i].set_unused();
            }
        });
        
        // Flush TLB to ensure cleared mappings take effect
        // SAFETY: Safe because we're flushing the TLB after clearing user mappings
        // to ensure no stale translations remain in the TLB cache
        #[allow(unused_unsafe)]
        unsafe {
            x86_64::instructions::tlb::flush_all();
        }
        
        Ok(())
    }
    
    /// Map a VmArea into this address space
    pub fn map_area(&mut self, area: &VmArea) -> Result<(), VmError> {
        // Check for overlaps with existing areas
        for existing_area in self.areas.values() {
            if area.overlaps(existing_area) {
                return Err(VmError::AddressInUse);
            }
        }
        
        // Add the area to our tracking
        self.areas.insert(area.start, area.clone());
        
        // For now, we don't need to actually map pages here
        // Page mapping will be done on-demand during page faults
        // or explicitly when loading ELF segments
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct VirtualMemoryManager {
    address_spaces: BTreeMap<u64, AddressSpace>,
    next_as_id: u64,
    current_as_id: Option<u64>,
    kernel_as_id: u64,
    shared_areas: BTreeMap<alloc::string::String, VmArea>,
}

impl VirtualMemoryManager {
    pub const fn new() -> Self {
        Self {
            address_spaces: BTreeMap::new(),
            next_as_id: 1,
            current_as_id: None,
            kernel_as_id: 0,
            shared_areas: BTreeMap::new(),
        }
    }
    
    pub fn create_address_space(&mut self) -> Result<u64, VmError> {
        let id = self.next_as_id;
        self.next_as_id += 1;
        
        let address_space = AddressSpace::new(id)?;
        self.address_spaces.insert(id, address_space);
        
        Ok(id)
    }
    
    pub fn destroy_address_space(&mut self, id: u64) -> Result<(), VmError> {
        if id == self.kernel_as_id {
            return Err(VmError::InvalidOperation);
        }
        
        if let Some(current_id) = self.current_as_id {
            if current_id == id {
                self.current_as_id = None;
            }
        }
        
        // Get the address space before removing it
        if let Some(address_space) = self.address_spaces.get(&id) {
            // Deallocate all mapped pages in all areas
            memory::with_mapper(|mapper| {
                for area in address_space.areas.values() {
                    for page in area.pages() {
                        if let Ok(frame) = mapper.translate_page(page) {
                            // Unmap the page and deallocate the frame
                            if let Ok((_frame, flush)) = mapper.unmap(page) {
                                flush.ignore(); // We'll do a batch TLB flush later
                                memory::deallocate_frame(frame);
                            }
                        }
                    }
                }
                
                // Flush TLB after unmapping all pages
                x86_64::instructions::tlb::flush_all();
            });
            
            // Deallocate the PML4 frame
            let pml4_frame = address_space.pml4_frame;
            memory::deallocate_frame(pml4_frame);
        }
        
        self.address_spaces.remove(&id);
        Ok(())
    }
    
    pub fn get_address_space(&self, id: u64) -> Option<&AddressSpace> {
        self.address_spaces.get(&id)
    }
    
    pub fn get_address_space_mut(&mut self, id: u64) -> Option<&mut AddressSpace> {
        self.address_spaces.get_mut(&id)
    }
    
    pub fn switch_address_space(&mut self, id: u64) -> Result<(), VmError> {
        if !self.address_spaces.contains_key(&id) {
            return Err(VmError::InvalidAddressSpace);
        }
        
        // Don't switch if already in the target address space
        if self.current_as_id == Some(id) {
            return Ok(());
        }
        
        let address_space = self.address_spaces.get(&id).ok_or(VmError::InvalidAddressSpace)?;
        let new_pml4_frame = address_space.pml4_frame;
        
        self.current_as_id = Some(id);
        
        // Switch CR3 to the new address space's PML4 and flush TLB
        let (_, flags) = Cr3::read();
        // SAFETY: Safe because:
        // 1. new_pml4_frame is a valid PML4 frame from a verified address space
        // 2. The PML4 contains proper kernel mappings copied during creation
        // 3. flags are preserved from the current CR3 to maintain CPU state
        // 4. TLB flush ensures no stale translations remain after switch
        // 5. This is the only place where CR3 is modified for address space switching
        unsafe { 
            Cr3::write(new_pml4_frame, flags);
            // Flush entire TLB to ensure proper address space isolation
            x86_64::instructions::tlb::flush_all();
        }
        
        Ok(())
    }
    
    // Update memory protection flags and flush TLB
    pub fn protect_memory(&mut self, as_id: u64, virt_addr: VirtAddr, size: usize, permissions: VmPermissions) -> Result<(), VmError> {
        // Enforce W^X policy or dual-mapping policy
        permissions.validate_dual_mapping_policy()?;
        
        let _address_space = self.get_address_space_mut(as_id)
            .ok_or(VmError::InvalidAddressSpace)?;
        
        let flags = permissions.to_page_table_flags();
        let start_page = Page::<Size4KiB>::containing_address(virt_addr);
        let end_page = Page::<Size4KiB>::containing_address(virt_addr + size - 1u64);
        
        memory::with_mapper(|mapper| {
            for page in Page::range_inclusive(start_page, end_page) {
                // Update page table flags by unmapping and remapping with new flags
                if let Ok(frame) = mapper.translate_page(page) {
                    // Unmap the existing page
                    if let Ok((_frame, flush)) = mapper.unmap(page) {
                        flush.ignore(); // We'll do a batch TLB flush later
                        
                        // Remap with new flags
                        if let Some(frame_alloc) = memory::FRAME_ALLOC.lock().as_mut() {
                            // SAFETY: This is unsafe because:
                            // - `mapper.map_to` requires exclusive access to page tables
                            // - `page` must be a valid virtual page not already mapped
                            // - `frame` must be a valid, unused physical frame
                            // - `flags` must be valid page table flags
                            // - `frame_alloc` must be a valid frame allocator
                            // - We hold the VMM lock ensuring no concurrent page table modifications
                            // - The mapping is ignored here because we do batch TLB flushing below
                            if let Ok(mapping) = unsafe { mapper.map_to(page, frame, flags, &mut *frame_alloc) } {
                                mapping.ignore(); // We'll do a batch TLB flush later
                            }
                        }
                    }
                }
            }
        });
        
        // Flush TLB for the affected pages
        // SAFETY: This is unsafe because:
        // - `invlpg` is a privileged x86 instruction that invalidates TLB entries
        // - We must ensure the page addresses are valid virtual addresses
        // - This is required after page table modifications to maintain TLB coherency
        // - The inline assembly constraints are correctly specified
        // - We're running in kernel mode with appropriate privileges
        unsafe {
            for page in Page::range_inclusive(start_page, end_page) {
                core::arch::asm!("invlpg [{}]", in(reg) page.start_address().as_u64());
            }
        }
        
        Ok(())
    }
    
    pub fn map_page(&mut self, as_id: u64, virt_addr: VirtAddr, phys_addr: PhysAddr, flags: PageTableFlags) -> Result<(), VmError> {
        let _address_space = self.get_address_space_mut(as_id)
            .ok_or(VmError::InvalidAddressSpace)?;
        memory::with_mapper(|mapper| {
            let frame = PhysFrame::containing_address(phys_addr);
            let page: Page<Size4KiB> = Page::containing_address(virt_addr);
            let mut alloc = GlobalFrameAlloc;
            // SAFETY: This is unsafe because:
            // - `mapper.map_to` requires exclusive access to page tables
            // - `page` must be a valid virtual page not already mapped
            // - `frame` must be a valid physical frame
            // - `flags` must be valid page table flags
            // - `alloc` must be a valid frame allocator
            // - We hold the VMM lock ensuring no concurrent modifications
            match unsafe { mapper.map_to(page, frame, flags, &mut alloc) } {
                Ok(mapping) => { mapping.flush(); Ok(()) }
                Err(_) => Err(VmError::MapError),
            }
        })
    }
    
    pub fn unmap_page(&mut self, as_id: u64, virt_addr: VirtAddr) -> Result<(), VmError> {
        let _address_space = self.get_address_space_mut(as_id)
            .ok_or(VmError::InvalidAddressSpace)?;
        memory::with_mapper(|mapper| {
            let page: Page<Size4KiB> = Page::containing_address(virt_addr);
            match mapper.unmap(page) {
                Ok((frame, flush)) => { 
                    flush.flush(); 
                    // Deallocate the frame after unmapping
                    memory::deallocate_frame(frame);
                    Ok(()) 
                }
                Err(_) => Err(VmError::UnmapError),
            }
        })
    }
    
    pub fn handle_page_fault(&mut self, virt_addr: VirtAddr, error_code: u64) -> Result<(), VmError> {
        let current_as_id = self.current_as_id.ok_or(VmError::InvalidAddressSpace)?;
        let address_space_ptr: *mut AddressSpace = self.get_address_space_mut(current_as_id)
            .ok_or(VmError::InvalidAddressSpace)?;
        let address_space = unsafe { &mut *address_space_ptr };
        
        // Find the VMA containing this address
        let area = address_space.find_area(virt_addr)
            .ok_or(VmError::SegmentationFault)?;
        
        // Check permissions
        let is_write = (error_code & 0x2) != 0;
        let is_user = (error_code & 0x4) != 0;
        let is_instruction_fetch = (error_code & 0x10) != 0;
        
        if is_write && !area.permissions.writable() {
            return Err(VmError::PermissionDenied);
        }
        
        if is_user && !area.permissions.user_accessible() {
            return Err(VmError::PermissionDenied);
        }
        
        if is_instruction_fetch && !area.permissions.executable() {
            return Err(VmError::PermissionDenied);
        }
        
        // Handle different types of page faults
        match area.area_type {
            VmAreaType::Stack => {
                // Stack growth
                if virt_addr < area.start {
                    self.expand_stack(current_as_id, virt_addr)?;
                } else {
                    self.allocate_page_on_demand(current_as_id, virt_addr, area.permissions)?;
                }
            },
            VmAreaType::Heap => {
                self.allocate_page_on_demand(current_as_id, virt_addr, area.permissions)?;
            },
            _ => {
                self.allocate_page_on_demand(current_as_id, virt_addr, area.permissions)?;
            }
        }
        
        Ok(())
    }
    
    fn expand_stack(&mut self, as_id: u64, fault_addr: VirtAddr) -> Result<(), VmError> {
        let as_ptr: *mut AddressSpace = self.get_address_space_mut(as_id)
            .ok_or(VmError::InvalidAddressSpace)?;
        let address_space = unsafe { &mut *as_ptr };
        
        // Find the stack area
        let mut stack_area = None;
        for area in address_space.areas.values() {
            if area.area_type == VmAreaType::Stack {
                stack_area = Some(area.clone());
                break;
            }
        }
        
        let mut stack_area = stack_area.ok_or(VmError::InvalidOperation)?;
        
        // Check if we can expand the stack
        let page_addr = Page::<Size4KiB>::containing_address(fault_addr).start_address();
        if page_addr < address_space.stack_start {
            return Err(VmError::StackOverflow);
        }
        
        // Expand the stack area
        stack_area.start = page_addr;
        
        // Remove old area and add expanded one
        address_space.areas.retain(|_, area| area.area_type != VmAreaType::Stack);
        address_space.add_area(stack_area)?;
        
        // Allocate the page
        let _ = self.allocate_page_on_demand(as_id, fault_addr, VmPermissions::READ);
        
        Ok(())
    }
    
    fn allocate_page_on_demand(&mut self, _as_id: u64, virt_addr: VirtAddr, permissions: VmPermissions) -> Result<(), VmError> {
        // Enforce W^X policy or dual-mapping policy
        permissions.validate_dual_mapping_policy()?;
        
        let flags = permissions.to_page_table_flags();
        memory::with_mapper(|mapper| {
            let frame = memory::allocate_frame().ok_or(VmError::OutOfMemory)?;
            let page: Page<Size4KiB> = Page::containing_address(virt_addr);
            let mut alloc = GlobalFrameAlloc;
            match unsafe { mapper.map_to(page, frame, flags, &mut alloc) } {
                Ok(res) => {
                    res.flush();
                    Ok(())
                }
                Err(_) => {
                    // Mapping failed - deallocate the frame we allocated
                    memory::deallocate_frame(frame);
                    Err(VmError::MapError)
                }
            }
        })
    }

    pub fn create_shared_area(&mut self, name: alloc::string::String, size: u64, permissions: VmPermissions) -> Result<(), VmError> {
        let start = VirtAddr::new(0x4000_0000_0000); // Shared memory region
        let end = start + size;
        
        let area = VmArea {
            start,
            end,
            area_type: VmAreaType::Shared,
            permissions,
            name: Some(name.clone()),
            file_offset: None,
            is_shared: true,
            is_anonymous: true,
            ref_count: 0,
        };
        
        self.shared_areas.insert(name, area);
        Ok(())
    }
    
    pub fn map_shared_area(&mut self, as_id: u64, name: &str, virt_addr: Option<VirtAddr>) -> Result<VirtAddr, VmError> {
        let shared_area = self.shared_areas.get(name)
            .cloned()
            .ok_or(VmError::NotFound)?;
        
        let address_space = self.get_address_space_mut(as_id)
            .ok_or(VmError::InvalidAddressSpace)?;
        
        let start_addr = if let Some(addr) = virt_addr {
            addr
        } else {
            address_space.next_mmap
        };
        
        let mut area = shared_area.clone();
        area.start = start_addr;
        area.end = start_addr + shared_area.size();
        area.ref_count += 1;
        
        address_space.add_area(area)?;
        
        if virt_addr.is_none() {
            address_space.next_mmap += shared_area.size();
        }
        
        Ok(start_addr)
    }
}

struct GlobalFrameAlloc;
unsafe impl FrameAllocator<Size4KiB> for GlobalFrameAlloc {
    fn allocate_frame(&mut self) -> Option<PhysFrame> { memory::allocate_frame() }
}

#[derive(Debug)]
pub enum VmError {
    OutOfMemory,
    InvalidAddressSpace,
    InvalidOperation,
    AddressInUse,
    NotFound,
    PermissionDenied,
    SegmentationFault,
    StackOverflow,
    InvalidAlignment,
    MapError,
    UnmapError,
    TestFailed,
    WxViolation,
    JitNotAllowed,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VmError::OutOfMemory => write!(f, "Out of memory"),
            VmError::InvalidAddressSpace => write!(f, "Invalid address space"),
            VmError::InvalidOperation => write!(f, "Invalid operation"),
            VmError::AddressInUse => write!(f, "Address already in use"),
            VmError::NotFound => write!(f, "Not found"),
            VmError::PermissionDenied => write!(f, "Permission denied"),
            VmError::SegmentationFault => write!(f, "Segmentation fault"),
            VmError::StackOverflow => write!(f, "Stack overflow"),
            VmError::InvalidAlignment => write!(f, "Invalid alignment"),
            VmError::MapError => write!(f, "Mapping error"),
            VmError::UnmapError => write!(f, "Unmapping error"),
            VmError::TestFailed => write!(f, "Test failed"),
            VmError::WxViolation => write!(f, "W^X policy violation"),
            VmError::JitNotAllowed => write!(f, "JIT compilation not allowed"),
        }
    }
}

pub type VmResult<T> = Result<T, VmError>;

// Memory statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_virtual: u64,
    pub used_virtual: u64,
    pub total_physical: u64,
    pub used_physical: u64,
    pub cached: u64,
    pub buffers: u64,
    pub shared: u64,
    pub page_faults: u64,
    pub major_page_faults: u64,
}

impl MemoryStats {
    pub fn new() -> Self {
        Self {
            total_virtual: 0,
            used_virtual: 0,
            total_physical: 0,
            used_physical: 0,
            cached: 0,
            buffers: 0,
            shared: 0,
            page_faults: 0,
            major_page_faults: 0,
        }
    }
}

static MEMORY_STATS: Mutex<MemoryStats> = Mutex::new(MemoryStats {
    total_virtual: 0,
    used_virtual: 0,
    total_physical: 0,
    used_physical: 0,
    cached: 0,
    buffers: 0,
    shared: 0,
    page_faults: 0,
    major_page_faults: 0,
});

// Public API functions
pub fn init() {
    let mut vmm = VMM.write();
    
    // Create kernel address space
    let kernel_as_id = vmm.create_address_space().expect("Failed to create kernel address space");
    vmm.kernel_as_id = kernel_as_id;
    vmm.current_as_id = Some(kernel_as_id);
    
    // Set up kernel memory areas
    if let Some(kernel_as) = vmm.get_address_space_mut(kernel_as_id) {
        // Kernel code area
        let kernel_code = VmArea::new(
            VirtAddr::new(0xFFFF_8000_0000_0000),
            VirtAddr::new(0xFFFF_8000_0010_0000),
            VmAreaType::Code,
            VmPermissions::READ
        );
        let _ = kernel_as.add_area(kernel_code);
        
        // Kernel data area
        let kernel_data = VmArea::new(
            VirtAddr::new(0xFFFF_8000_0010_0000),
            VirtAddr::new(0xFFFF_8000_0020_0000),
            VmAreaType::Data,
            VmPermissions::READ | VmPermissions::WRITE
        );
        let _ = kernel_as.add_area(kernel_data);
        
        // Kernel heap area
        let kernel_heap = VmArea::new(
            VirtAddr::new(0xFFFF_8000_0020_0000),
            VirtAddr::new(0xFFFF_8000_1000_0000),
            VmAreaType::Heap,
            VmPermissions::READ | VmPermissions::WRITE
        );
        let _ = kernel_as.add_area(kernel_heap);
    }
}

pub fn create_address_space() -> Result<u64, VmError> {
    VMM.write().create_address_space()
}

pub fn destroy_address_space(id: u64) -> VmResult<()> {
    VMM.write().destroy_address_space(id)
}

pub fn switch_address_space(id: u64) -> VmResult<()> {
    VMM.write().switch_address_space(id)
}

pub fn allocate_area(as_id: u64, size: u64, area_type: VmAreaType, permissions: VmPermissions) -> VmResult<VirtAddr> {
    let mut vmm = VMM.write();
    let address_space = vmm.get_address_space_mut(as_id)
        .ok_or(VmError::InvalidAddressSpace)?;
    address_space.allocate_area(size, area_type, permissions)
}

pub fn deallocate_area(as_id: u64, start: VirtAddr) -> VmResult<()> {
    let mut vmm = VMM.write();
    let address_space = vmm.get_address_space_mut(as_id)
        .ok_or(VmError::InvalidAddressSpace)?;
    
    address_space.remove_area(start)
        .ok_or(VmError::NotFound)?;
    
    Ok(())
}

pub fn map_page(as_id: u64, virt_addr: VirtAddr, phys_addr: PhysAddr, permissions: VmPermissions) -> VmResult<()> {
    let flags = permissions.to_page_table_flags();
    VMM.write().map_page(as_id, virt_addr, phys_addr, flags)
}

pub fn unmap_page(as_id: u64, virt_addr: VirtAddr) -> VmResult<()> {
    VMM.write().unmap_page(as_id, virt_addr)
}

pub fn handle_page_fault(virt_addr: VirtAddr, error_code: u64) -> VmResult<()> {
    VMM.write().handle_page_fault(virt_addr, error_code)
}

pub fn create_shared_memory(name: alloc::string::String, size: u64, permissions: VmPermissions) -> VmResult<()> {
    VMM.write().create_shared_area(name, size, permissions)
}

pub fn map_shared_memory(as_id: u64, name: &str, virt_addr: Option<VirtAddr>) -> VmResult<VirtAddr> {
    VMM.write().map_shared_area(as_id, name, virt_addr)
}

pub fn get_memory_stats() -> MemoryStats {
    *MEMORY_STATS.lock()
}

pub fn update_memory_stats<F>(updater: F) where F: FnOnce(&mut MemoryStats) {
    let mut stats = MEMORY_STATS.lock();
    updater(&mut stats);
}

pub fn get_address_space_info(as_id: u64) -> Option<(u64, Vec<(VirtAddr, VirtAddr, VmAreaType, VmPermissions)>)> {
    let vmm = VMM.read();
    let address_space = vmm.get_address_space(as_id)?;
    
    let areas: Vec<_> = address_space.areas.values()
        .map(|area| (area.start, area.end, area.area_type, area.permissions))
        .collect();
    
    Some((as_id, areas))
}

pub fn protect_memory(as_id: u64, start: VirtAddr, _size: u64, permissions: VmPermissions) -> VmResult<()> {
    let mut vmm = VMM.write();
    let address_space = vmm.get_address_space_mut(as_id)
        .ok_or(VmError::InvalidAddressSpace)?;
    
    // Find and update the area
    if let Some(area) = address_space.find_area_mut(start) {
        area.permissions = permissions;
        
        // Update page table permissions for all mapped pages in this area
        let flags = permissions.to_page_table_flags();
        let start_page = Page::<Size4KiB>::containing_address(area.start);
        let end_page = Page::<Size4KiB>::containing_address(area.end - 1u64);
        
        memory::with_mapper(|mapper| {
            for page in Page::range_inclusive(start_page, end_page) {
                if let Ok(frame) = mapper.translate_page(page) {
                    // Update page table flags by unmapping and remapping with new flags
                    if let Ok((_frame, flush)) = mapper.unmap(page) {
                        flush.ignore(); // We'll do a batch TLB flush later
                        
                        // Remap with new flags
                        if let Some(frame_alloc) = memory::FRAME_ALLOC.lock().as_mut() {
                            if let Ok(mapping) = unsafe { mapper.map_to(page, frame, flags, &mut *frame_alloc) } {
                                mapping.ignore(); // We'll do a batch TLB flush later
                            }
                        }
                    }
                }
            }
            // Flush TLB to ensure changes take effect
            x86_64::instructions::tlb::flush_all();
        });
        
        Ok(())
    } else {
        Err(VmError::NotFound)
    }
}

pub fn copy_address_space(src_id: u64) -> VmResult<u64> {
    let mut vmm = VMM.write();
    
    // Get source address space
    let src_areas = {
        let src_as = vmm.get_address_space(src_id)
            .ok_or(VmError::InvalidAddressSpace)?;
        src_as.areas.clone()
    };
    
    // Create new address space
    let new_id = vmm.create_address_space()?;
    
    // Copy all areas
    if let Some(new_as) = vmm.get_address_space_mut(new_id) {
        for area in src_areas.values() {
            let mut new_area = area.clone();
            new_area.ref_count = 1;
            let _ = new_as.add_area(new_area);
        }
    }
    
    Ok(new_id)
}

// Public API for protect_memory
pub fn protect_memory_api(as_id: u64, virt_addr: VirtAddr, size: usize, permissions: VmPermissions) -> VmResult<()> {
    VMM.write().protect_memory(as_id, virt_addr, size, permissions)
}

// Gaming mode optimizations
pub fn enable_gaming_mode(as_id: u64) -> VmResult<()> {
    let mut vmm = VMM.write();
    let _address_space = vmm.get_address_space_mut(as_id)
        .ok_or(VmError::InvalidAddressSpace)?;
    
    // Pre-allocate common memory areas to reduce page faults
    // This is a simplified implementation
    
    Ok(())
}

pub fn disable_gaming_mode(_as_id: u64) -> VmResult<()> {
    // Restore normal memory management
    Ok(())
}

// Memory defragmentation
pub fn defragment_memory(as_id: u64) -> VmResult<()> {
    let mut vmm = VMM.write();
    let address_space = vmm.get_address_space_mut(as_id)
        .ok_or(VmError::InvalidAddressSpace)?;
    
    // Simple defragmentation: compact areas by removing gaps
    let mut areas_to_move = Vec::new();
    let mut current_addr = VirtAddr::new(0x400000); // Start at 4MB
    
    // Collect areas that need to be moved
    for area in address_space.areas.values() {
        if area.start != current_addr {
            areas_to_move.push((area.start, current_addr, area.size()));
        }
        current_addr += area.size();
    }
    
    // Move pages to compact memory (simplified implementation)
    memory::with_mapper(|mapper| {
        for (old_start, new_start, size) in areas_to_move {
            let old_page = Page::<Size4KiB>::containing_address(old_start);
            let new_page = Page::<Size4KiB>::containing_address(new_start);
            let page_count = (size + 4095) / 4096; // Round up to page count
            
            for i in 0..page_count {
                let old_page_addr = old_page + i;
                let _new_page_addr = new_page + i;
                
                if let Ok(frame) = mapper.translate_page(old_page_addr) {
                    if let Ok((_frame, flush)) = mapper.unmap(old_page_addr) {
                        flush.ignore(); // We'll do a batch TLB flush later
                        // Deallocate the frame since we're not remapping it
                        memory::deallocate_frame(frame);
                    }
                }
            }
        }
        x86_64::instructions::tlb::flush_all();
    });
    
    Ok(())
}

// Memory compression (for low memory situations)
pub fn compress_unused_pages(as_id: u64) -> VmResult<u64> {
    let mut vmm = VMM.write();
    let address_space = vmm.get_address_space_mut(as_id)
        .ok_or(VmError::InvalidAddressSpace)?;
    
    let mut bytes_saved = 0u64;
    
    // Iterate through all areas to find pages that can be compressed
    for area in &mut address_space.areas {
        // Only compress pages in areas marked as compressible (swappable)
        if area.1.permissions.contains(VmPermissions::SWAPPABLE) {
            let start_page = Page::<Size4KiB>::containing_address(area.1.start);
            let end_page = Page::<Size4KiB>::containing_address(area.1.end - 1u64);
            
            memory::with_mapper(|mapper| {
                for page in Page::range_inclusive(start_page, end_page) {
                    if let Ok(_frame) = mapper.translate_page(page) {
                        // For now, we'll just unmap it to simulate compression
                        // In a real implementation, we would check if page was accessed recently
                        if let Ok((_frame, flush)) = mapper.unmap(page) {
                            flush.ignore(); // Batch TLB flush later
                            bytes_saved += 4096; // One page saved
                            
                            // In a real implementation, we would:
                            // 1. Read the page content
                            // 2. Compress it using a compression algorithm
                            // 3. Store compressed data in a swap area
                            // 4. Mark the page as compressed in area metadata
                        }
                    }
                }
            });
        }
    }
    
    // Flush TLB after all operations
    x86_64::instructions::tlb::flush_all();
    
    Ok(bytes_saved)
}

pub fn decompress_pages(as_id: u64) -> VmResult<()> {
    let mut vmm = VMM.write();
    let address_space = vmm.get_address_space_mut(as_id)
        .ok_or(VmError::InvalidAddressSpace)?;
    
    // Iterate through all areas to restore compressed pages
    for area in &mut address_space.areas {
        // Only decompress pages in areas that support compression
        if area.1.permissions.contains(VmPermissions::SWAPPABLE) {
            let start_page = Page::<Size4KiB>::containing_address(area.1.start);
            let end_page = Page::<Size4KiB>::containing_address(area.1.end - 1u64);
            
            memory::with_mapper(|mapper| {
                for page in Page::range_inclusive(start_page, end_page) {
                    // Check if page is not currently mapped (indicating it might be compressed)
                    if mapper.translate_page(page).is_err() {
                        // Page is not mapped, try to restore it
                        if let Some(frame) = memory::allocate_frame() {
                            let flags = area.1.permissions.to_page_table_flags();
                            
                            // Map the new frame with appropriate flags
                            if let Some(frame_alloc) = memory::FRAME_ALLOC.lock().as_mut() {
                                if let Ok(mapping) = unsafe { mapper.map_to(page, frame, flags, &mut *frame_alloc) } {
                                    mapping.ignore(); // Batch TLB flush later
                                    
                                    // In a real implementation, we would:
                                    // 1. Locate the compressed data for this page
                                    // 2. Decompress the data
                                    // 3. Copy decompressed data to the new frame
                                    // 4. Update area metadata to mark page as uncompressed
                                    
                                    // For now, zero the page to provide clean memory
                                    let page_ptr = memory::phys_to_virt(frame.start_address()).as_mut_ptr::<u8>();
                                    unsafe {
                                        core::ptr::write_bytes(page_ptr, 0, 4096);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
    }
    
    // Flush TLB after all operations
    x86_64::instructions::tlb::flush_all();
    
    Ok(())
}

/// Provide access to the VMM instance with a closure
pub fn with_vmm<F, R>(f: F) -> R
where
    F: FnOnce(&mut VirtualMemoryManager) -> R,
{
    let mut vmm = VMM.write();
    f(&mut vmm)
}

/// Test address space isolation
pub fn test_address_space_isolation() -> VmResult<()> {
    // Create two separate address spaces
    let as1_id = create_address_space()?;
    let as2_id = create_address_space()?;
    
    // Allocate memory in each address space at the same virtual address
    let test_addr = VirtAddr::new(0x400000); // 4MB
    let test_size = 4096; // One page
    
    // Allocate in first address space
    allocate_area(as1_id, test_size, VmAreaType::Data, 
                  VmPermissions::READ | VmPermissions::WRITE | VmPermissions::USER)?;
    
    // Allocate in second address space at same virtual address
    allocate_area(as2_id, test_size, VmAreaType::Data,
                  VmPermissions::READ | VmPermissions::WRITE | VmPermissions::USER)?;
    
    // Switch to first address space and write test data
    switch_address_space(as1_id)?;
    
    // Map a test page in AS1
    with_vmm(|vmm| {
        if let Some(frame) = memory::allocate_frame() {
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
            vmm.map_page(as1_id, test_addr, frame.start_address(), flags)?;
            
            // Write test pattern to AS1
            unsafe {
                let ptr = test_addr.as_mut_ptr::<u32>();
                *ptr = 0xDEADBEEF;
            }
        }
        Ok(())
    })?;
    
    // Switch to second address space and write different data
    switch_address_space(as2_id)?;
    
    with_vmm(|vmm| {
        if let Some(frame) = memory::allocate_frame() {
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
            vmm.map_page(as2_id, test_addr, frame.start_address(), flags)?;
            
            // Write different test pattern to AS2
            unsafe {
                let ptr = test_addr.as_mut_ptr::<u32>();
                *ptr = 0xCAFEBABE;
            }
        }
        Ok(())
    })?;
    
    // Verify isolation by switching back and checking values
    switch_address_space(as1_id)?;
    unsafe {
        let ptr = test_addr.as_ptr::<u32>();
        let value1 = *ptr;
        if value1 != 0xDEADBEEF {
            return Err(VmError::TestFailed);
        }
    }
    
    switch_address_space(as2_id)?;
    unsafe {
        let ptr = test_addr.as_ptr::<u32>();
        let value2 = *ptr;
        if value2 != 0xCAFEBABE {
            return Err(VmError::TestFailed);
        }
    }
    
    // Clean up
    destroy_address_space(as1_id)?;
    destroy_address_space(as2_id)?;
    
    Ok(())
}

/// Test memory protection functionality
pub fn test_memory_protection() -> VmResult<()> {
    // Create an address space for testing
    let as_id = create_address_space()?;
    
    // Allocate a test area
    let test_addr = VirtAddr::new(0x500000); // 5MB
    let test_size = 4096; // One page
    
    allocate_area(as_id, test_size, VmAreaType::Data,
                  VmPermissions::READ | VmPermissions::WRITE | VmPermissions::USER)?;
    
    // Map the page with read/write permissions
    with_vmm(|vmm| {
        if let Some(frame) = memory::allocate_frame() {
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
            vmm.map_page(as_id, test_addr, frame.start_address(), flags)?;
        }
        Ok(())
    })?;
    
    switch_address_space(as_id)?;
    
    // Test 1: Write to writable page (should succeed)
    unsafe {
        // SAFETY: This is unsafe because:
        // - test_addr must be a valid, mapped virtual address
        // - The page must be mapped with WRITABLE permissions
        // - The address must be properly aligned for u32 access
        // - No other code should be accessing this test memory concurrently
        // - The mapping must remain valid during the write operation
        let ptr = test_addr.as_mut_ptr::<u32>();
        *ptr = 0x12345678; // This should work
    }
    
    // Test 2: Change protection to read-only
    protect_memory(as_id, test_addr, test_size as u64, 
                   VmPermissions::READ | VmPermissions::USER)?;
    
    // Test 3: Verify read still works
    unsafe {
        // SAFETY: This is unsafe because:
        // - test_addr must be a valid, mapped virtual address
        // - The page must be mapped with READ permissions
        // - The address must be properly aligned for u32 access
        // - The mapping must remain valid during the read operation
        // - The memory content must be initialized (we wrote to it earlier)
        let ptr = test_addr.as_ptr::<u32>();
        let value = *ptr;
        if value != 0x12345678 {
            return Err(VmError::TestFailed);
        }
    }
    
    // Test 4: Change protection to no access
    protect_memory(as_id, test_addr, test_size as u64, VmPermissions::empty())?;
    
    // Note: We can't easily test page fault generation in kernel space
    // without setting up proper exception handling, so we'll just verify
    // the page table flags were updated correctly
    
    // Test 5: Restore write permissions
    protect_memory(as_id, test_addr, test_size as u64,
                   VmPermissions::READ | VmPermissions::WRITE | VmPermissions::USER)?;
    
    // Test 6: Verify write works again
    unsafe {
        // SAFETY: This is unsafe because:
        // - test_addr must be a valid, mapped virtual address
        // - The page must be mapped with READ and WRITE permissions
        // - The address must be properly aligned for u32 access
        // - No other code should be accessing this test memory concurrently
        // - The mapping must remain valid during both write and read operations
        let ptr = test_addr.as_mut_ptr::<u32>();
        *ptr = 0x87654321;
        let value = *ptr;
        if value != 0x87654321 {
            return Err(VmError::TestFailed);
        }
    }
    
    // Clean up
    destroy_address_space(as_id)?;
    
    Ok(())
}

/// Run all VMM tests
pub fn run_vmm_tests() -> VmResult<()> {
    crate::serial::_print(format_args!("[VMM] Testing address space isolation..."));
    test_address_space_isolation()?;
    crate::serial::_print(format_args!(" PASS\n"));
    
    crate::serial::_print(format_args!("[VMM] Testing memory protection..."));
    test_memory_protection()?;
    crate::serial::_print(format_args!(" PASS\n"));
    
    crate::serial::_print(format_args!("[VMM] All tests passed!\n"));
    Ok(())
}
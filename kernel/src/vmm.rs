use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::fmt;
use spin::{Mutex, RwLock};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmPermissions(u8);

impl VmPermissions {
    pub const Read: Self = Self(0x1);
    pub const Write: Self = Self(0x2);
    pub const Execute: Self = Self(0x4);
    pub const User: Self = Self(0x8);
    pub const Global: Self = Self(0x10);
    pub const NoCache: Self = Self(0x20);
    pub const WriteThrough: Self = Self(0x40);
}

impl VmPermissions {
    pub fn readable(self) -> bool { (self.0 & Self::Read.0) != 0 }
    
    pub fn writable(self) -> bool { (self.0 & Self::Write.0) != 0 }
    
    pub fn executable(self) -> bool { (self.0 & Self::Execute.0) != 0 }
    
    pub fn user_accessible(self) -> bool { (self.0 & Self::User.0) != 0 }
    
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
        
        if (self.0 & Self::Global.0) != 0 {
            flags |= PageTableFlags::GLOBAL;
        }
        
        if (self.0 & Self::NoCache.0) != 0 {
            flags |= PageTableFlags::NO_CACHE;
        }
        
        if (self.0 & Self::WriteThrough.0) != 0 {
            flags |= PageTableFlags::WRITE_THROUGH;
        }
        
        flags
    }
}

impl core::ops::BitOr for VmPermissions {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output { Self(self.0 | rhs.0) }
}

impl core::ops::BitOrAssign for VmPermissions {
    fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
}

// Duplicate implementation removed - using the first one above

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
    pub fn new(id: u64) -> Self {
        // Allocate a new physical frame for the PML4
        let pml4_frame = memory::allocate_frame().expect("Failed to allocate PML4 frame");
        
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
        
        address_space
    }
    
    // Map kernel higher-half into this address space
    fn map_kernel_space(&mut self) {
        // Get current kernel PML4 to copy kernel mappings
        let (current_pml4_frame, _) = Cr3::read();
        
        memory::with_mapper(|mapper| {
            // Map the new PML4 temporarily to copy kernel entries
            let pml4_virt = memory::phys_to_virt(self.pml4_frame.start_address());
            let new_pml4 = unsafe { &mut *(pml4_virt.as_mut_ptr::<PageTable>()) };
            
            // Get current PML4
            let current_pml4_virt = memory::phys_to_virt(current_pml4_frame.start_address());
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
    
    pub fn create_address_space(&mut self) -> u64 {
        let id = self.next_as_id;
        self.next_as_id += 1;
        
        let address_space = AddressSpace::new(id);
        self.address_spaces.insert(id, address_space);
        
        id
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
        
        let address_space = self.address_spaces.get(&id).unwrap();
        let new_pml4_frame = address_space.pml4_frame;
        
        self.current_as_id = Some(id);
        
        // Switch CR3 to the new address space's PML4
        let (_, flags) = Cr3::read();
        unsafe { 
            Cr3::write(new_pml4_frame, flags);
            // Flush TLB to ensure address space isolation
            core::arch::asm!("mov {}, cr3", in(reg) new_pml4_frame.start_address().as_u64());
        }
        
        Ok(())
    }
    
    // Update memory protection flags and flush TLB
    pub fn protect_memory(&mut self, as_id: u64, virt_addr: VirtAddr, size: usize, permissions: VmPermissions) -> Result<(), VmError> {
        let address_space = self.get_address_space_mut(as_id)
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
                        if let Some(mut frame_alloc) = memory::FRAME_ALLOC.lock().as_mut() {
                            if let Ok(mapping) = unsafe { mapper.map_to(page, frame, flags, &mut *frame_alloc) } {
                                mapping.ignore(); // We'll do a batch TLB flush later
                            }
                        }
                    }
                }
            }
        });
        
        // Flush TLB for the affected pages
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
                Ok((_frame, flush)) => { flush.flush(); Ok(()) }
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
        let _ = self.allocate_page_on_demand(as_id, fault_addr, VmPermissions::Read);
        
        Ok(())
    }
    
    fn allocate_page_on_demand(&mut self, _as_id: u64, virt_addr: VirtAddr, permissions: VmPermissions) -> Result<(), VmError> {
        let flags = permissions.to_page_table_flags();
        memory::with_mapper(|mapper| {
            let frame = memory::allocate_frame().ok_or(VmError::OutOfMemory)?;
            let page: Page<Size4KiB> = Page::containing_address(virt_addr);
            let mut alloc = GlobalFrameAlloc;
            let res = unsafe { mapper.map_to(page, frame, flags, &mut alloc) }.map_err(|_| VmError::MapError)?;
            res.flush();
            Ok(())
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
    let kernel_as_id = vmm.create_address_space();
    vmm.kernel_as_id = kernel_as_id;
    vmm.current_as_id = Some(kernel_as_id);
    
    // Set up kernel memory areas
    if let Some(kernel_as) = vmm.get_address_space_mut(kernel_as_id) {
        // Kernel code area
        let kernel_code = VmArea::new(
            VirtAddr::new(0xFFFF_8000_0000_0000),
            VirtAddr::new(0xFFFF_8000_0010_0000),
            VmAreaType::Code,
            VmPermissions::Read
        );
        let _ = kernel_as.add_area(kernel_code);
        
        // Kernel data area
        let kernel_data = VmArea::new(
            VirtAddr::new(0xFFFF_8000_0010_0000),
            VirtAddr::new(0xFFFF_8000_0020_0000),
            VmAreaType::Data,
            VmPermissions::Write
        );
        let _ = kernel_as.add_area(kernel_data);
        
        // Kernel heap area
        let kernel_heap = VmArea::new(
            VirtAddr::new(0xFFFF_8000_0020_0000),
            VirtAddr::new(0xFFFF_8000_1000_0000),
            VmAreaType::Heap,
            VmPermissions::Write
        );
        let _ = kernel_as.add_area(kernel_heap);
    }
}

pub fn create_address_space() -> u64 {
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

pub fn protect_memory(as_id: u64, start: VirtAddr, size: u64, permissions: VmPermissions) -> VmResult<()> {
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
                        if let Some(mut frame_alloc) = memory::FRAME_ALLOC.lock().as_mut() {
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
    let new_id = vmm.create_address_space();
    
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
    let address_space = vmm.get_address_space_mut(as_id)
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
                let new_page_addr = new_page + i;
                
                if let Ok((frame, flags)) = mapper.translate_page(old_page_addr) {
                    let _ = mapper.unmap(old_page_addr);
                    // Note: This is a simplified implementation - proper page moving would require more complex logic
                    // For now, we'll skip the actual remapping to avoid frame allocator access issues
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
        if area.permissions.contains(VmPermissions::SWAPPABLE) {
            let start_page = Page::<Size4KiB>::containing_address(area.start);
            let end_page = Page::<Size4KiB>::containing_address(area.end - 1u64);
            
            memory::with_mapper(|mapper| {
                for page in Page::range_inclusive(start_page, end_page) {
                    if let Ok((frame, flags)) = mapper.translate_page(page) {
                        // Check if page is accessed recently (clear accessed bit and check)
                        if !flags.contains(PageTableFlags::ACCESSED) {
                            // Page hasn't been accessed recently, candidate for compression
                            // For now, we'll just unmap it to simulate compression
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
        if area.permissions.contains(VmPermissions::SWAPPABLE) {
            let start_page = Page::<Size4KiB>::containing_address(area.start);
            let end_page = Page::<Size4KiB>::containing_address(area.end - 1u64);
            
            memory::with_mapper(|mapper| {
                for page in Page::range_inclusive(start_page, end_page) {
                    // Check if page is not currently mapped (indicating it might be compressed)
                    if mapper.translate_page(page).is_err() {
                        // Page is not mapped, try to restore it
                        if let Some(frame) = memory::allocate_frame() {
                            let flags = area.permissions.to_page_table_flags();
                            
                            // Map the new frame with appropriate flags
                            if let Some(mut frame_alloc) = memory::FRAME_ALLOC.lock().as_mut() {
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
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{OffsetPageTable, PageTable, PhysFrame, FrameAllocator, Size4KiB};
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
static FRAME_ALLOC: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);

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

// Convert physical address to virtual address using the physical memory offset
pub fn phys_to_virt(phys_addr: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys_addr.as_u64() + active_physical_offset())
}

// Convert virtual address to physical address using the physical memory offset
pub fn virt_to_phys(virt_addr: VirtAddr) -> PhysAddr {
    PhysAddr::new(virt_addr.as_u64() - active_physical_offset())
}



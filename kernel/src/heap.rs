// no alloc imports needed here
use linked_list_allocator::LockedHeap;
use x86_64::structures::paging::{Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::{VirtAddr};

// Place heap well above kernel code/data mapping to avoid overlaps
pub const HEAP_START: usize = 0x_4444_0000_0000;
pub const HEAP_SIZE: usize = 1024 * 1024; // 1 MiB

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub fn init_heap<M, F>(
    mapper: &mut M,
    frame_allocator: &mut F,
) -> Result<(), &'static str>
where
    M: Mapper<Size4KiB>,
    F: x86_64::structures::paging::FrameAllocator<Size4KiB>,
{
    let heap_start = VirtAddr::new(HEAP_START as u64);
    let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;
    let start_page = Page::containing_address(heap_start);
    let end_page = Page::containing_address(heap_end);
    
    for page in Page::range_inclusive(start_page, end_page) {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or("frame allocation failed")?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        
        unsafe {
            // Handle the case where page might already be mapped
            match mapper.map_to(page, frame, flags, frame_allocator) {
                Ok(flush) => flush.flush(),
                Err(x86_64::structures::paging::mapper::MapToError::PageAlreadyMapped(_)) => {
                    // Page is already mapped, which is fine for heap initialization
                    // Just continue to the next page
                    continue;
                },
                Err(_) => return Err("map_to failed"),
            }
        }
    }
    
    unsafe { ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE) }
    Ok(())
}



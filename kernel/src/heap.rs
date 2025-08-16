use linked_list_allocator::LockedHeap;
use x86_64::structures::paging::{Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::{VirtAddr};
use alloc::vec::Vec;

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
    
    let mut allocated_frames = Vec::new();
    
    for page in Page::range_inclusive(start_page, end_page) {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or("frame allocation failed")?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        
        // SAFETY: This is unsafe because:
        // - page must be a valid virtual page that is not currently mapped
        // - frame must be a valid, unused physical frame
        // - flags must be appropriate for heap memory (PRESENT | WRITABLE)
        // - mapper must be a valid page table mapper
        // - frame_allocator must be a valid frame allocator for additional page tables
        // - We handle the PageAlreadyMapped case to avoid double-mapping
        unsafe {
            // Handle the case where page might already be mapped
            match mapper.map_to(page, frame, flags, frame_allocator) {
                Ok(flush) => {
                    flush.flush();
                    allocated_frames.push((page, frame));
                }
                Err(x86_64::structures::paging::mapper::MapToError::PageAlreadyMapped(_)) => {
                    // Page is already mapped, which is fine for heap initialization
                    // Deallocate the frame we allocated since we don't need it
                    crate::memory::deallocate_frame(frame);
                    continue;
                },
                Err(_) => {
                    // Mapping failed - clean up all previously allocated frames
                    for (allocated_page, allocated_frame) in allocated_frames {
                        let _ = mapper.unmap(allocated_page);
                        crate::memory::deallocate_frame(allocated_frame);
                    }
                    // Also deallocate the current frame
                    crate::memory::deallocate_frame(frame);
                    return Err("map_to failed");
                },
            }
        }
    }
    
    // SAFETY: This is unsafe because:
    // - HEAP_START must point to valid, mapped, writable memory
    // - HEAP_SIZE must not exceed the actually mapped memory region
    // - The memory region [HEAP_START, HEAP_START + HEAP_SIZE) must be exclusively owned by the allocator
    // - This must only be called once during system initialization
    // - All pages in the heap range have been successfully mapped above
    unsafe { ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE) }
    Ok(())
}



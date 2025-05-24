//! # Proka Kernel - A kernel for ProkaOS
//! Copyright (C) RainSTR Studio 2025, All rights reserved.
//!
//! This file contains the idea of alllocating the heap memory
//! and the global allocator.
/* Modules use area */
use core::alloc::{GlobalAlloc, Layout};
use linked_list_allocator::LockedHeap;
use spin::Mutex;
use x86_64::structures::paging::*;
use x86_64::{PhysAddr, VirtAddr};

/* First, declare the heap's start and size */
/// The beginning address of the heap.
const HEAP_START: usize = 0x105000;

/// The size of the heap.
const HEAP_SIZE: usize = 64 * 1024 * 1024; // 16M available

/* Then, declare an global allocator */
#[global_allocator]
static ALLOCATOR: LocalHeapAllocator = LocalHeapAllocator(Mutex::new(LockedHeap::empty()));

/* The frame allocator */
pub struct FrameAlloc {
    next_frame: PhysAddr,
}

impl FrameAlloc {
    fn new(start_addr: PhysAddr) -> Self {
        Self {
            next_frame: start_addr,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for FrameAlloc {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Alloc that frame
        let frame = PhysFrame::containing_address(self.next_frame);

        // Move to the next frame address
        self.next_frame += Size4KiB::SIZE;

        Some(frame)
    }
}

/* The local heap allocator */
struct LocalHeapAllocator(Mutex<LockedHeap>);

/* And finish the mapper (The paging has enabled in long mode)*/
/// Map the physical memory
pub fn map_heap_mem(mapper: &mut impl Mapper<Size4KiB>, phys_start: u64) {
    /* Initialize */
    // The virtual address start
    let virt_start = VirtAddr::new(HEAP_START as u64);

    // The physical address start
    let phys_start = PhysAddr::new(phys_start);

    // The end of the virtual memory
    let virt_end = virt_start + HEAP_SIZE as u64;

    // The start of the page
    let page_start = Page::containing_address(virt_start);

    // The end of the page
    let page_end = Page::containing_address(virt_end - 1); // Avoid overflow

    // The range of page
    let page_range = Page::range_inclusive(page_start, page_end);

    // Initialize the frame allocator
    let mut frame_allocator = FrameAlloc::new(phys_start);

    // Check add address in heap
    for page in page_range {
        // Get the frame
        let page_offset = page - page_start;
        let frame_phys_addr = phys_start + page_offset * Size4KiB::SIZE;
        let frame = PhysFrame::containing_address(frame_phys_addr); // Convert to PhysFrame

        // Set up the flag
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        // Finally map the virtual to physics
        unsafe {
            mapper
                .map_to(page, frame, flags, &mut frame_allocator)
                .unwrap()
                .flush()
        }
    }
}

/* The initializer of the memory */
pub fn init_heap() {
    unsafe {
        // Initialize locked heap
        let mut guard = ALLOCATOR.0.lock(); // Get lock
        *guard = LockedHeap::new(HEAP_START, HEAP_SIZE);
    }
}

unsafe impl GlobalAlloc for LocalHeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { self.0.lock().alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.0.lock().dealloc(ptr, layout) }
    }
}

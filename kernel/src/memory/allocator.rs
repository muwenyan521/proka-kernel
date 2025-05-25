//! # Proka Kernel - A kernel for ProkaOS
//! Copyright (C) RainSTR Studio 2025, All rights reserved.
//!
//! This file contains the idea of alllocating the heap memory
//! and the global allocator.
/* Modules use area */
use core::alloc::{GlobalAlloc, Layout};
use linked_list_allocator::LockedHeap;
use spin::Mutex;

/* First, declare the heap's start and size */
/// The beginning address of the heap.
const HEAP_START: usize = 0x105000;

/// The size of the heap.
const HEAP_SIZE: usize = 64 * 1024 * 1024; // 16M available

/* Then, declare an global allocator */
#[global_allocator]
static ALLOCATOR: LocalHeapAllocator = LocalHeapAllocator(Mutex::new(LockedHeap::empty()));

/* The local heap allocator */
struct LocalHeapAllocator(Mutex<LockedHeap>);

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

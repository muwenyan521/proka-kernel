extern crate alloc;
use crate::MEMORY_MAP_REQUEST;
use alloc::vec::Vec;
use bitmap_allocator::{BitAlloc, BitAlloc4K};
use lazy_static::lazy_static;
use limine::memory_map::EntryType;
use spin::Mutex;
use x86_64::PhysAddr;
use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB};

/// Convert Physical address to Physical Frame
fn addr_to_phys_frame(addr: Option<usize>) -> Option<PhysFrame> {
    let addr = addr?;
    let phys_addr = PhysAddr::new(addr as u64);
    PhysFrame::from_start_address(phys_addr).ok()
}

lazy_static! {
    pub static ref GLOBAL_FRAME_ALLOCATOR: PhysFrameAlloc = {
        let mut frame_allocator = PhysFrameAlloc::new();
        let memory_map = MEMORY_MAP_REQUEST.get_response().unwrap();

        // Collact all usable memory region
        let mut usable_ranges = Vec::new();
        for entry in memory_map.entries().iter() {
            if entry.entry_type == EntryType::USABLE {
                let start = entry.base as usize;
                let end = (entry.base + entry.length) as usize;
                usable_ranges.push((start, end));
            }
        }

        // Initialize the frame allocator in ONLY 1 TIME
        frame_allocator.init(&usable_ranges);
        frame_allocator
    };
}

pub struct PhysFrameAlloc {
    /// The range of the physical frame allocator
    range: Vec<(usize, usize)>,
    /// The bitmap allocator
    inner: Mutex<BitAlloc4K>,
    /// Whether the allocator is initialized
    initialized: Mutex<bool>,
}

impl PhysFrameAlloc {
    pub fn new() -> Self {
        Self {
            range: Vec::new(),
            inner: Mutex::new(BitAlloc4K::default()),
            initialized: Mutex::new(false),
        }
    }

    /// The initializator of physical frame allocator.
    pub fn init(&mut self, ranges: &[(usize, usize)]) {
        self.range = ranges.to_vec();
        let mut guard = self.inner.lock();
        for &(start, end) in ranges {
            guard.insert(start..end); // Add available range to bitmap allocator
        }
        *self.initialized.lock() = true;
    }

    fn is_initialized(&self) -> bool {
        *self.initialized.lock()
    }

    pub fn alloc_frame(&self) -> Option<PhysFrame> {
        if !self.is_initialized() {
            return None;
        }
        let result = self.inner.lock().alloc();
        addr_to_phys_frame(result)
    }

    pub fn dealloc_frame(&self, key: usize) {
        if !self.is_initialized() {
            return;
        }
        // Check if the key is within the initialized range
        let mut is_in_managed_range = false;
        for &(start, end) in &self.range {
            // The physical frame is 4KiB aligned, so check here
            if key >= start && key < end && (key % 0x1000 == 0) {
                is_in_managed_range = true;
                break;
            }
        }
        if !is_in_managed_range {
            // the key is NOT in the range of the frame allocator, just ignore
            return;
        }

        let mut inner = self.inner.lock();
        if inner.test(key) {
            inner.dealloc(key);
        } else {
            // This frame was not allocated, maybe log an error
            // Temporary solution: panic
            panic!("Deallocating unallocated frame");
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for PhysFrameAlloc {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.alloc_frame()
    }
}

impl FrameDeallocator<Size4KiB> for PhysFrameAlloc {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame) {
        let start_addr: PhysAddr = frame.start_address();
        let addr = start_addr.as_u64() as usize;
        self.dealloc_frame(addr);
    }
}

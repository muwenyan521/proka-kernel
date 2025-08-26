use bitmap_allocator::{BitAlloc4K, BitAlloc};
use spin::Mutex;
use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB};
use x86_64::PhysAddr;

/// Convert Physical address to Physical Frame
fn addr_to_phys_frame(addr: Option<usize>) -> Option<PhysFrame> {
    // Return None if the addr is None
    let addr = addr?;

    // Convert addr to Physical addr
    let phys_addr = PhysAddr::new(addr as u64);

    // Try to build PhysFrame from start address
    PhysFrame::from_start_address(phys_addr).ok()
}

pub struct PhysFrameAlloc {
    /// The base address
    base: usize,

    /// The bitmap allocator
    inner: Mutex<BitAlloc4K>,
}

impl PhysFrameAlloc {
    pub fn new() -> Self {
        Self {
            base: 0,
            inner: Mutex::new(BitAlloc4K::default()),
        }
    }

    /// The initializator of physical frame allocator.
    pub fn init(&mut self, start: usize, end: usize) {
        self.base = start;
        let mut guard = self.inner.lock();
        guard.insert(start..end);
    }

    pub fn alloc_frame(&self) -> Option<PhysFrame> {
        let result = self.inner.lock().alloc();
        addr_to_phys_frame(result)
    }
    pub fn dealloc_frame(&self, key: usize) {
        let _ = self.inner.lock().dealloc(key);
    }
}

// Implement the FrameAllocator trait
unsafe impl FrameAllocator<Size4KiB> for PhysFrameAlloc {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
       self.alloc_frame() 
    }
}

impl FrameDeallocator<Size4KiB> for PhysFrameAlloc {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame) {
        let start_addr: PhysAddr = frame.start_address();
        // Convert to u64 and convert to usize forcely
        let addr = start_addr.as_u64() as usize;
        let _ = self.dealloc_frame(addr);
    }
}

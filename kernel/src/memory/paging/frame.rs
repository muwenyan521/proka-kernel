use bitmap_allocator::{BitAlloc, BitAlloc4K};
use spin::Mutex;
use x86_64::PhysAddr;
use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB};

/// Convert Physical address to Physical Frame
fn addr_to_phys_frame(addr: Option<usize>) -> Option<PhysFrame> {
    let addr = addr?;
    let phys_addr = PhysAddr::new(addr as u64);
    PhysFrame::from_start_address(phys_addr).ok()
}

pub struct PhysFrameAlloc {
    /// The base address
    base: usize,
    /// The end address
    end: usize,
    /// The bitmap allocator
    inner: Mutex<BitAlloc4K>,
    /// Whether the allocator is initialized
    initialized: Mutex<bool>,
}

impl PhysFrameAlloc {
    pub fn new() -> Self {
        Self {
            base: 0,
            end: 0,
            inner: Mutex::new(BitAlloc4K::default()),
            initialized: Mutex::new(false),
        }
    }

    /// The initializator of physical frame allocator.
    pub fn init(&mut self, start: usize, end: usize) {
        self.base = start;
        self.end = end;
        let mut guard = self.inner.lock();
        guard.insert(start..end);
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
        if key < self.base || key >= self.end {
            // This address is not managed by this allocator
            return;
        }
        let mut inner = self.inner.lock();
        if inner.test(key) {
            inner.dealloc(key);
        } else {
            // This frame was not allocated, maybe log an error
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

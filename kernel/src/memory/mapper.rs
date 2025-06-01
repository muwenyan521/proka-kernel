extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use multiboot2::{MemoryAreaType, MemoryMapTag};
use spin::{Mutex, Once}; // For safe global initialization
use x86_64::structures::paging::*;
use x86_64::{PhysAddr, VirtAddr};

// Global frame allocator (safe initialization)
static FRAME_ALLOCATOR: Once<LockedFrameAllocator> = Once::new();

// Global memory mapper (safe initialization)
static MEMORY_MAPPER: Once<LockedMemoryMapper> = Once::new();

/// Initialize the global frame allocator
pub fn init_frame_allocator(memmap: &MemoryMapTag) {
    FRAME_ALLOCATOR.call_once(|| LockedFrameAllocator::new(memmap));
}

/// Initialize the global memory mapper
pub fn init_memory_mapper(physical_memory_offset: VirtAddr) {
    MEMORY_MAPPER.call_once(|| LockedMemoryMapper::new(physical_memory_offset));
}

/// Map physical page address to virtual address
pub fn map_physical_page(physical_addr: u64, virtual_addr: u64) {
    if let Some(mapper) = MEMORY_MAPPER.get() {
        mapper.lock().map_page(
            physical_addr,
            virtual_addr,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        );
    }
}

/* Frame Allocator with locking */
pub struct LockedFrameAllocator {
    inner: Mutex<FrameAlloc>,
}

impl LockedFrameAllocator {
    /// Create a new frame allocator from memory map
    pub fn new(mmap_tag: &MemoryMapTag) -> Self {
        let frame_alloc = FrameAlloc::init(mmap_tag);
        LockedFrameAllocator {
            inner: Mutex::new(frame_alloc),
        }
    }

    /// Lock the allocator for frame allocation
    pub fn lock(&self) -> spin::MutexGuard<FrameAlloc> {
        self.inner.lock()
    }
}

/* The actual frame allocator implementation */
pub struct FrameAlloc {
    frames: Vec<PhysFrame>,
}

impl FrameAlloc {
    /// Initialize the physical frame allocator
    pub fn init(mmap_tag: &MemoryMapTag) -> Self {
        // Get all available memory regions
        let available_regions = mmap_tag
            .memory_areas()
            .into_iter()
            .filter(|area| area.typ() == MemoryAreaType::Available)
            .filter(|area| area.start_address() >= 0x100000);

        let mut frames = Vec::new();

        // Convert each available region to physical frames
        for region in available_regions {
            let start_frame = PhysFrame::containing_address(PhysAddr::new(region.start_address()));
            let end_frame = PhysFrame::containing_address(PhysAddr::new(region.end_address() - 1));

            for frame in PhysFrame::range_inclusive(start_frame, end_frame) {
                frames.push(frame);
            }
        }

        // Optional: Randomize frame order for security
        // frames.shuffle(&mut rand::thread_rng());

        FrameAlloc { frames }
    }
}

// Implement FrameAllocator trait for FrameAlloc
unsafe impl FrameAllocator<Size4KiB> for FrameAlloc {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.frames.pop()
    }
}

/* Thread-safe memory mapper */
pub struct LockedMemoryMapper {
    inner: spin::Mutex<MemoryMapper>,
}

impl LockedMemoryMapper {
    /// Create a new memory mapper
    pub fn new(physical_memory_offset: VirtAddr) -> Self {
        // Get level 4 table pointer from physical memory offset
        let level_4_table_ptr = physical_memory_offset.as_ptr::<PageTable>() as *mut PageTable;

        // SAFETY: This is safe during early boot when we're setting up paging
        let level_4_table = unsafe { &mut *level_4_table_ptr };

        // Create OffsetPageTable
        let mapper = unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) };

        LockedMemoryMapper {
            inner: spin::Mutex::new(MemoryMapper { mapper }),
        }
    }

    /// Lock the mapper for page table operations
    pub fn lock(&self) -> spin::MutexGuard<MemoryMapper> {
        self.inner.lock()
    }
}

/* The actual memory mapper implementation */
pub struct MemoryMapper {
    mapper: OffsetPageTable<'static>,
}

impl MemoryMapper {
    /// Map a physical address to a virtual address
    pub fn map_page(&mut self, physical_addr: u64, virtual_addr: u64, flags: PageTableFlags) {
        let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virtual_addr));
        let frame = PhysFrame::containing_address(PhysAddr::new(physical_addr));

        // Get global frame allocator
        let frame_alloc = FRAME_ALLOCATOR
            .get()
            .expect("Frame allocator not initialized");
        let mut frame_allocator = frame_alloc.lock();

        unsafe {
            self.mapper
                .map_to(page, frame, flags, &mut *frame_allocator)
                .expect("Mapping failed")
                .flush();
        }
    }
}

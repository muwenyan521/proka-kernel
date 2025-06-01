extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use multiboot2::{MemoryAreaType, MemoryMapTag};
use spin::{Mutex, Once}; // For safe global initialization
use x86_64::registers::control::Cr3;
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

/// Map continuous physical address to virtual address
pub fn map_continuous_page(physical_base: u64, virtual_base: u64, size: u64) {
    if let Some(mapper) = MEMORY_MAPPER.get() {
        mapper.lock().map_continuous_memory(
            physical_base,
            virtual_base,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
            size
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
        let (level_4_table_frame, _) = Cr3::read();
        let level_4_table_phys = level_4_table_frame.start_address();
        let level_4_table_virt = physical_memory_offset + level_4_table_phys.as_u64();
        let level_4_table_ptr = level_4_table_virt.as_mut_ptr::<PageTable>();

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

    /// Map the continuous physical addr to virtual one
    pub fn map_continuous_memory(&mut self, virt_base: u64, phys_base: u64, flags: PageTableFlags, size: u64) {
        let mut virt_addr = virt_base;
        let mut phys_addr = phys_base;
        let page_size = 4096;   // 4KiB page

        // Compute the page should map
        for _ in 0..(size + page_size - 1) / page_size {
            self.map_page(phys_addr, virt_addr, flags);

            virt_addr = virt_addr + page_size;
            phys_addr = phys_addr + page_size;
        }
    }
}

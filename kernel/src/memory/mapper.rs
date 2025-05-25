use x86_64::structures::paging::*;
use x86_64::{PhysAddr, VirtAddr};

// Global static variable to store memory mapper
static mut MEMORY_MAPPER: Option<MemoryMapper> = None;

const FRAME_BITMAP_SIZE: usize = 1024 * 1024 / 4096 / 8; // 1MB size

static mut FRAME_BITMAP: [u8; FRAME_BITMAP_SIZE] = [0; FRAME_BITMAP_SIZE];
static mut NEXT_FREE_FRAME: usize = 0;

/// The initializer of the memory mapper (single)
pub fn init_memory_mapper() {
    unsafe {
        MEMORY_MAPPER = Some(MemoryMapper::new());
    }
}

/// Map physical page address to the virtual one ()single
pub fn map_physical_page(physical_addr: u64, virtual_addr: u64) {
    unsafe {
        if let Some(ref mut mapper) = MEMORY_MAPPER {
            mapper.map_page(physical_addr, virtual_addr, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        }
    }
}

/* The frame allocator */
pub struct FrameAlloc;

impl FrameAlloc {
    /// Initialize the physical frame
    pub fn init(&mut self) {
        unsafe {
            // Initialize all the bitmap
            FRAME_BITMAP = [0; FRAME_BITMAP_SIZE];
            NEXT_FREE_FRAME = 0;
        }
    }

    fn frame_to_bit(frame: PhysFrame<Size4KiB>) -> usize {
        frame.start_address().as_u64() as usize / Size4KiB::SIZE as usize
    }

    fn bit_to_frame(bit: usize) -> PhysFrame<Size4KiB> {
        PhysFrame::from_start_address(PhysAddr::new(bit as u64 * Size4KiB::SIZE)).unwrap()
    }
}

unsafe impl FrameAllocator<Size4KiB> for FrameAlloc {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        unsafe {
            // Search for next free frame
            while NEXT_FREE_FRAME < FRAME_BITMAP_SIZE * 8 {
                let byte = NEXT_FREE_FRAME / 8;
                let bit = NEXT_FREE_FRAME % 8;
                if (FRAME_BITMAP[byte] & (1 << bit)) == 0 {
                    // Assign it to allocated
                    FRAME_BITMAP[byte] |= 1 << bit;
                    let frame = Self::bit_to_frame(NEXT_FREE_FRAME);
                    NEXT_FREE_FRAME += 1;
                    return Some(frame);
                }
                NEXT_FREE_FRAME += 1;
            }
            None // No free frame
        }
    }
}

/// The memory mapper manager
pub struct MemoryMapper<'a> {
    mapper: OffsetPageTable<'a>,
}

impl MemoryMapper<'_> {
    /// Initialize the memory mapper
    pub fn new() -> Self {
        let physical_memory_offset = VirtAddr::new(0x100000); // Offsets

        let level_4_table_ptr = physical_memory_offset.as_ptr::<PageTable>() as *mut PageTable;
        let level_4_table = unsafe { &mut *(level_4_table_ptr as *mut PageTable) };

        MemoryMapper {
            mapper: unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) },
        }
    }

    /// Map the physical page address to the virtual address
    pub fn map_page(&mut self, physical_addr: u64, virtual_addr: u64, flags: PageTableFlags) {
        let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virtual_addr));
        let frame = PhysFrame::containing_address(PhysAddr::new(physical_addr));
        let mut frame_allocator = FrameAlloc;
        frame_allocator.init();

        unsafe {
            self.mapper
                .map_to(page, frame, flags, &mut frame_allocator)
                .unwrap()
                .flush();
        }
    }
}

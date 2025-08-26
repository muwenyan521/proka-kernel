use x86_64::structures::paging::{PageTableFlags, PhysFrame};
use x86_64::PhysAddr;

/// The standard 4KiB Page Size
pub const PAGE_SIZE: usize = 4096;

/// The Page Table Entry Address
#[repr(transparent)]
pub struct PageTableEntry(u64);

// Implementations
impl PageTableEntry {
    pub fn new(frame: PhysFrame, flags: PageTableFlags) -> Self {
        Self(frame.start_address().as_u64() | flags.bits())
    }

    pub fn is_present(&self) -> bool {
        self.0 & PageTableFlags::PRESENT.bits() != 0
    }

    pub fn frame(&self) -> PhysFrame {
        PhysFrame::containing_address(PhysAddr::new_truncate(self.0 & 0x000f_ffff_ffff_f000))
    }
}

//! Paging module for Proka Kernel
//! Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//!
//! This module provides paging support, including:
//! - HHDM (Higher Half Direct Map) offset retrieval
//! - OffsetPageTable initialization
//! - Frame allocator based on bootloader memory map
//! - Heap initialization

extern crate alloc;
use limine::memory_map::EntryType;
use limine::response::MemoryMapResponse;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

/// Retrieve the HHDM (Higher Half Direct Map) offset from Limine
///
/// This offset is used to map physical memory into virtual address space.
pub fn get_hhdm_offset() -> VirtAddr {
    VirtAddr::new(
        crate::HHDM_REQUEST
            .get_response()
            .expect("Failed to get HHDM response")
            .offset(),
    )
}

/// Initialize an OffsetPageTable for accessing page tables
///
/// # Arguments
/// * `physical_memory_offset` - The HHDM offset returned by bootloader
///
/// # Safety
/// This function is unsafe because the caller must guarantee that:
/// - The complete physical memory is mapped to virtual memory at the passed offset
/// - This function is only called once to avoid aliasing `&mut` references
pub unsafe fn init_offset_page_table(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

/// Returns a mutable reference to the active level 4 table.
///
/// # Safety
/// This function is unsafe because the caller must guarantee that:
/// - The complete physical memory is mapped to virtual memory at the passed offset
/// - This function is only called once to avoid aliasing `&mut` references
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

/// A FrameAllocator that returns usable frames from Limine bootloader memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMapResponse,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that:
    /// - The passed memory map is valid
    /// - All frames marked as `USABLE` in it are really unused
    pub unsafe fn new(memory_map: &'static MemoryMapResponse) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.entries();
        let usable_regions = regions.iter().filter(|r| r.entry_type == EntryType::USABLE);

        let addr_ranges = usable_regions.map(|r| {
            let start = r.base as usize;
            let end = (r.base + r.length) as usize;
            start..end
        });

        addr_ranges
            .flat_map(|r| r.step_by(4096))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr as u64)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

//! Bitmap-based physical frame allocator with deallocation support
//! Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//!
//! This module provides a frame allocator using the bitmap-allocator crate,
//! supporting both allocation and deallocation of physical frames.

extern crate alloc;

use bitmap_allocator::BitAlloc;
use limine::memory_map::EntryType;
use limine::response::MemoryMapResponse;
use spin::Mutex;
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};

/// The size of a page in bytes (4 KiB)
pub const PAGE_SIZE: usize = 4096;

/// Bitmap frame allocator type - supports up to 16M frames (64 GiB)
///
/// This type alias represents a bitmap allocator that can manage up to
/// 16,777,216 frames, which corresponds to 64 GiB of physical memory
/// (assuming 4 KiB pages).
type BitAlloc16M = bitmap_allocator::BitAlloc16M;

/// Global allocator instance
///
/// We use a `static` variable here because `BitmapFrameAllocator` contains
/// `BitAlloc16M` which is a very large struct (~2.2 MB). Creating it on
/// the stack would cause a stack overflow and kernel panic (triple fault).
static FRAME_ALLOCATOR_INNER: Mutex<BitmapFrameAllocator> = Mutex::new(BitmapFrameAllocator {
    alloc: BitAlloc16M::DEFAULT,
    total_frames: 0,
    used_frames: 0,
});

/// Frame statistics
///
/// Contains detailed information about physical memory usage.
#[derive(Debug, Clone, Copy)]
pub struct FrameStats {
    /// Total number of frames in the system
    pub total_frames: usize,
    /// Number of free frames available for allocation
    pub free_frames: usize,
    /// Number of frames currently in use
    pub used_frames: usize,
    /// Total physical memory in bytes
    pub total_memory: usize,
    /// Free physical memory in bytes
    pub free_memory: usize,
    /// Used physical memory in bytes
    pub used_memory: usize,
}

/// Bitmap-based frame allocator
///
/// Manages physical memory frames using a bitmap to track allocation status.
/// Each bit in the bitmap corresponds to one 4 KiB frame.
pub struct BitmapFrameAllocator {
    /// The bitmap allocator instance
    alloc: BitAlloc16M,
    /// Total number of frames managed by this allocator
    total_frames: usize,
    /// Number of frames currently allocated
    used_frames: usize,
}

impl BitmapFrameAllocator {
    /// Initialize the allocator from the memory map
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that:
    /// - The passed memory map is valid
    /// - All frames marked as `USABLE` in it are really unused
    pub unsafe fn init(&mut self, memory_map: &'static MemoryMapResponse) {
        // Mark all usable frames as available in the bitmap
        for region in memory_map.entries().iter() {
            if region.entry_type == EntryType::USABLE {
                let start = region.base as usize;
                let end = (region.base + region.length) as usize;
                let start_frame = start / PAGE_SIZE;
                let end_frame = (end + PAGE_SIZE - 1) / PAGE_SIZE;

                self.alloc.insert(start_frame..end_frame);
                self.total_frames += end_frame - start_frame;
            }
        }
    }

    /// Allocate a contiguous block of frames
    ///
    /// Attempts to allocate `count` physically contiguous frames.
    ///
    /// # Arguments
    /// * `count` - Number of contiguous frames to allocate
    ///
    /// # Returns
    /// * `Some(PhysFrame)` - The first frame of the allocated block if successful
    /// * `None` - If insufficient contiguous memory is available
    ///
    /// # Note
    /// If `count` is 1, this function delegates to `allocate_frame()`.
    pub fn allocate_contiguous(&mut self, count: usize) -> Option<PhysFrame> {
        if count == 1 {
            self.allocate_frame()
        } else {
            let frame_num = self.alloc.alloc_contiguous(None, count, 0)?;
            self.used_frames += count;
            Some(PhysFrame::containing_address(x86_64::PhysAddr::new(
                (frame_num * PAGE_SIZE) as u64,
            )))
        }
    }

    /// Deallocate a single frame
    ///
    /// Marks the specified frame as free in the bitmap.
    ///
    /// # Arguments
    /// * `frame` - The physical frame to deallocate
    ///
    /// # Note
    /// If the frame was not previously allocated, this function has no effect.
    pub fn deallocate_frame(&mut self, frame: PhysFrame) {
        let frame_num = frame.start_address().as_u64() as usize / PAGE_SIZE;
        if self.alloc.dealloc(frame_num) {
            self.used_frames -= 1;
        }
    }

    /// Deallocate a contiguous block of frames
    ///
    /// Marks a block of contiguous frames as free in the bitmap.
    ///
    /// # Arguments
    /// * `frame` - The first frame of the contiguous block
    /// * `count` - Number of contiguous frames to deallocate
    ///
    /// # Note
    /// If any of the frames in the range were not previously allocated,
    /// this function has no effect on those frames.
    pub fn deallocate_contiguous(&mut self, frame: PhysFrame, count: usize) {
        let frame_num = frame.start_address().as_u64() as usize / PAGE_SIZE;
        if self.alloc.dealloc_contiguous(frame_num, count) {
            self.used_frames -= count;
        }
    }

    /// Get memory statistics
    ///
    /// Returns a `FrameStats` structure containing detailed information
    /// about physical memory usage.
    ///
    /// # Returns
    /// A `FrameStats` instance with current memory statistics.
    pub fn stats(&self) -> FrameStats {
        FrameStats {
            total_frames: self.total_frames,
            free_frames: self.total_frames - self.used_frames,
            used_frames: self.used_frames,
            total_memory: self.total_frames * PAGE_SIZE,
            free_memory: (self.total_frames - self.used_frames) * PAGE_SIZE,
            used_memory: self.used_frames * PAGE_SIZE,
        }
    }

    /// Check if a frame is allocated
    ///
    /// # Arguments
    /// * `frame` - The physical frame to check
    ///
    /// # Returns
    /// * `true` - If the frame is currently allocated
    /// * `false` - If the frame is free
    pub fn is_allocated(&self, frame: PhysFrame) -> bool {
        let frame_num = frame.start_address().as_u64() as usize / PAGE_SIZE;
        !self.alloc.test(frame_num)
    }

    /// Get the number of free frames
    ///
    /// # Returns
    /// The number of frames currently available for allocation.
    pub fn free_frames(&self) -> usize {
        self.total_frames - self.used_frames
    }
}

unsafe impl FrameAllocator<Size4KiB> for BitmapFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame_num = self.alloc.alloc()?;
        self.used_frames += 1;
        Some(PhysFrame::containing_address(x86_64::PhysAddr::new(
            (frame_num * PAGE_SIZE) as u64,
        )))
    }
}

/// Global frame allocator with spinlock protection
///
/// Wrapper around a static mutex to avoid stack overflow.
/// This provides thread-safe access to the global frame allocator.
pub struct LockedFrameAllocator(&'static Mutex<BitmapFrameAllocator>);

impl LockedFrameAllocator {
    /// Initialize the global allocator from the memory map
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that:
    /// - The passed memory map is valid
    /// - All frames marked as `USABLE` in it are really unused
    /// - This is called only once during initialization
    pub unsafe fn init(memory_map: &'static MemoryMapResponse) -> Self {
        let mut allocator = FRAME_ALLOCATOR_INNER.lock();
        if allocator.total_frames == 0 {
            allocator.init(memory_map);
        }
        LockedFrameAllocator(&FRAME_ALLOCATOR_INNER)
    }

    /// Deallocate a frame
    ///
    /// Thread-safe version of `BitmapFrameAllocator::deallocate_frame`.
    ///
    /// # Arguments
    /// * `frame` - The physical frame to deallocate
    pub fn deallocate_frame(&self, frame: PhysFrame) {
        self.0.lock().deallocate_frame(frame);
    }

    /// Deallocate a contiguous block of frames
    ///
    /// Thread-safe version of `BitmapFrameAllocator::deallocate_contiguous`.
    ///
    /// # Arguments
    /// * `frame` - The first frame of the contiguous block
    /// * `count` - Number of contiguous frames to deallocate
    pub fn deallocate_contiguous(&self, frame: PhysFrame, count: usize) {
        self.0.lock().deallocate_contiguous(frame, count);
    }

    /// Get memory statistics
    ///
    /// Thread-safe version of `BitmapFrameAllocator::stats`.
    ///
    /// # Returns
    /// A `FrameStats` instance with current memory statistics.
    pub fn stats(&self) -> FrameStats {
        self.0.lock().stats()
    }

    /// Check if a frame is allocated
    ///
    /// Thread-safe version of `BitmapFrameAllocator::is_allocated`.
    ///
    /// # Arguments
    /// * `frame` - The physical frame to check
    ///
    /// # Returns
    /// * `true` - If the frame is currently allocated
    /// * `false` - If the frame is free
    pub fn is_allocated(&self, frame: PhysFrame) -> bool {
        self.0.lock().is_allocated(frame)
    }

    /// Get the number of free frames
    ///
    /// Thread-safe version of `BitmapFrameAllocator::free_frames`.
    ///
    /// # Returns
    /// The number of frames currently available for allocation.
    pub fn free_frames(&self) -> usize {
        self.0.lock().free_frames()
    }
}

unsafe impl FrameAllocator<Size4KiB> for LockedFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        self.0.lock().allocate_frame()
    }
}

/// Format byte count to human-readable string
///
/// Converts a byte count to a human-readable string with appropriate
/// binary unit prefix (KiB, MiB, GiB, TiB).
///
/// # Arguments
/// * `bytes` - The number of bytes to format
///
/// # Returns
/// A formatted string with the appropriate unit.
///
/// # Examples
/// ```
/// use kernel::memory::frame_allocator::format_bytes;
///
/// assert_eq!(format_bytes(1024), "1 KiB");
/// assert_eq!(format_bytes(1048576), "1 MiB");
/// assert_eq!(format_bytes(1073741824), "1 GiB");
/// ```
pub fn format_bytes(bytes: usize) -> alloc::string::String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes;
    let mut unit_index = 0;

    while size >= 1024 && unit_index < UNITS.len() - 1 {
        size /= 1024;
        unit_index += 1;
    }

    alloc::format!("{} {}", size, UNITS[unit_index])
}

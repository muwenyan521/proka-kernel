//! RAM-based block device implementation.
//!
//! This module provides a simple in-memory block device that simulates disk storage
//! using RAM. It's useful for testing filesystems, caching, and temporary storage
//! without requiring actual hardware.
//!
//! # Features
//!
//! - Configurable block size and capacity
//! - Thread-safe operations using [`RwLock`]
//! - Implements both [`SharedDeviceOps`] and [`BlockDevice`] traits
//! - Simple read/write operations with bounds checking
//!
//! # Examples
//!
//! ```rust
//! use crate::drivers::block::ramblk::RamBlockDevice;
//!
//! // Create a 1MB RAM disk with 512-byte blocks
//! let ramdisk = RamBlockDevice::new(2048, 512);
//! assert_eq!(ramdisk.num_blocks(), 2048);
//! assert_eq!(ramdisk.block_size(), 512);
//! ```
//!
//! # Limitations
//!
//! - Data is lost when the device is dropped (volatile storage)
//! - No persistence across reboots
//! - Limited to available system memory

use crate::drivers::device::{BlockDevice, DeviceError, DeviceType, SharedDeviceOps};
extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use spin::RwLock;

/// A RAM-based block device that stores data in memory.
///
/// This structure implements a virtual block device using system RAM as storage.
/// It's useful for testing, caching, and temporary storage scenarios where
/// persistence is not required.
///
/// # Fields
/// * `name` - Device name (always "ramdisk")
/// * `storage` - Thread-safe storage buffer protected by [`RwLock`]
/// * `block_size` - Size of each block in bytes
///
/// # Thread Safety
///
/// The device uses [`RwLock`] for thread-safe access, allowing multiple
/// concurrent readers or a single writer at any time.
#[allow(dead_code)]
pub struct RamBlockDevice {
    name: String,
    storage: RwLock<Vec<u8>>,
    block_size: usize,
}

impl RamBlockDevice {
    /// Creates a new RAM block device with the specified capacity.
    ///
    /// # Arguments
    /// * `num_blocks` - Number of blocks in the device
    /// * `block_size` - Size of each block in bytes
    ///
    /// # Returns
    /// A new `RamBlockDevice` instance with all blocks initialized to zero.
    ///
    /// # Examples
    /// ```rust
    /// use crate::drivers::block::ramblk::RamBlockDevice;
    ///
    /// // Create a 1MB RAM disk (2048 blocks × 512 bytes)
    /// let ramdisk = RamBlockDevice::new(2048, 512);
    /// ```
    #[allow(dead_code)]
    pub fn new(num_blocks: usize, block_size: usize) -> Self {
        Self {
            name: "ramdisk".to_string(),
            storage: RwLock::new(vec![0; num_blocks * block_size]),
            block_size,
        }
    }
}

impl SharedDeviceOps for RamBlockDevice {
    /// Returns the name of this RAM block device.
    ///
    /// # Returns
    /// Always returns "ramdisk".
    fn name(&self) -> &str {
        &self.name
    }

    /// Returns the type of this device.
    ///
    /// # Returns
    /// Always returns [`DeviceType::Block`] since this is a block device.
    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    /// Opens the device for operations.
    ///
    /// For RAM block devices, opening always succeeds as there's no
    /// hardware initialization required.
    ///
    /// # Returns
    /// Always returns `Ok(())`.
    fn open(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// Closes the device.
    ///
    /// For RAM block devices, closing always succeeds.
    ///
    /// # Returns
    /// Always returns `Ok(())`.
    fn close(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// Performs a device-specific control operation.
    ///
    /// RAM block devices do not support any control operations.
    ///
    /// # Arguments
    /// * `_cmd` - Command code (ignored)
    /// * `_arg` - Command argument (ignored)
    ///
    /// # Returns
    /// Always returns `Err(DeviceError::NotSupported)`.
    fn ioctl(&self, _cmd: u64, _arg: u64) -> Result<u64, DeviceError> {
        Err(DeviceError::NotSupported)
    }
}

impl BlockDevice for RamBlockDevice {
    /// Returns the size of each block in bytes.
    ///
    /// # Returns
    /// The block size specified when the device was created.
    fn block_size(&self) -> usize {
        self.block_size
    }

    /// Returns the total number of blocks available on the device.
    ///
    /// # Returns
    /// The total capacity divided by the block size.
    fn num_blocks(&self) -> usize {
        self.storage.read().len() / self.block_size
    }

    /// Reads one or more blocks from the device.
    ///
    /// # Arguments
    /// * `block_idx` - Starting block index (0-based)
    /// * `num_blocks` - Number of blocks to read
    /// * `buf` - Buffer to store read data (must be at least `num_blocks * block_size()` bytes)
    ///
    /// # Returns
    /// - `Ok(usize)` with number of blocks actually read (always equals `num_blocks` on success)
    /// - `Err(DeviceError::AddressOutOfRange)` if the requested blocks are outside device bounds
    /// - `Err(DeviceError::BufferTooSmall)` if the buffer is too small for the requested data
    ///
    /// # Examples
    /// ```rust
    /// use crate::drivers::block::ramblk::RamBlockDevice;
    ///
    /// let ramdisk = RamBlockDevice::new(10, 512);
    /// let mut buffer = [0u8; 1024]; // 2 blocks
    /// let result = ramdisk.read_blocks(0, 2, &mut buffer);
    /// assert!(result.is_ok());
    /// ```
    fn read_blocks(
        &self,
        block_idx: usize,
        num_blocks: usize,
        buf: &mut [u8],
    ) -> Result<usize, DeviceError> {
        let start = block_idx * self.block_size;
        let end = start + num_blocks * self.block_size;
        let storage = self.storage.read();

        if end > storage.len() {
            return Err(DeviceError::AddressOutOfRange);
        }

        if buf.len() < end - start {
            return Err(DeviceError::BufferTooSmall);
        }

        buf[..end - start].copy_from_slice(&storage[start..end]);
        Ok(num_blocks)
    }

    /// Writes one or more blocks to the device.
    ///
    /// # Arguments
    /// * `block_idx` - Starting block index (0-based)
    /// * `num_blocks` - Number of blocks to write
    /// * `buf` - Buffer containing data to write (must be at least `num_blocks * block_size()` bytes)
    ///
    /// # Returns
    /// - `Ok(usize)` with number of blocks actually written (always equals `num_blocks` on success)
    /// - `Err(DeviceError::AddressOutOfRange)` if the requested blocks are outside device bounds
    /// - `Err(DeviceError::InvalidParam)` if the buffer is too small for the requested data
    ///
    /// # Examples
    /// ```rust
    /// use crate::drivers::block::ramblk::RamBlockDevice;
    ///
    /// let ramdisk = RamBlockDevice::new(10, 512);
    /// let data = [0xFFu8; 1024]; // 2 blocks of 0xFF
    /// let result = ramdisk.write_blocks(0, 2, &data);
    /// assert!(result.is_ok());
    /// ```
    fn write_blocks(
        &self,
        block_idx: usize,
        num_blocks: usize,
        buf: &[u8],
    ) -> Result<usize, DeviceError> {
        let start = block_idx * self.block_size;
        let end = start + num_blocks * self.block_size;
        let mut storage = self.storage.write();

        if end > storage.len() {
            return Err(DeviceError::AddressOutOfRange);
        }

        if buf.len() < end - start {
            return Err(DeviceError::InvalidParam);
        }

        storage[start..end].copy_from_slice(&buf[..end - start]);
        Ok(num_blocks)
    }
}

//! Block I/O cache implementation
//!
//! This module provides a simple block cache for block devices. It caches
//! recently accessed blocks in memory to improve I/O performance by reducing
//! the number of actual device reads and writes.
//!
//! # Overview
//!
//! The `BlockCache` struct wraps a block device and provides transparent
//! caching of block data. When a block is read, it's first checked in the
//! cache. If not present, it's read from the underlying device and stored
//! in the cache for future access.
//!
//! # Design
//!
//! - Uses a B-tree map for O(log n) lookups
//! - Thread-safe through `RwLock` synchronization
//! - Write-through caching: writes go to both cache and device
//! - Simple LRU-like behavior (though not true LRU)
//!
//! # Usage
//!
//! ```rust
//! use crate::fs::bio::BlockCache;
//! use crate::drivers::device::BlockDevice;
//! use alloc::sync::Arc;
//!
//! // Assuming you have a block device
//! let device: Arc<dyn BlockDevice> = /* get block device */;
//! let cache = BlockCache::new(device);
//!
//! // Read a block (cached on first access)
//! if let Some(data) = cache.read_block(0) {
//!     // Use the block data
//! }
//!
//! // Write a block (cached and written to device)
//! let data = vec![0u8; 512];
//! cache.write_block(0, &data);
//! ```
//!
//! # Limitations
//!
//! - No cache eviction policy (cache grows indefinitely)
//! - No dirty block tracking (write-through only)
//! - No cache size limits
//!
//! # Future Improvements
//!
//! - Implement LRU or other eviction policies
//! - Add cache size limits
//! - Support write-back caching with dirty flags
//! - Add cache statistics and monitoring

extern crate alloc;
use crate::drivers::device::BlockDevice;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

/// Block cache for improving I/O performance
///
/// This struct provides a caching layer for block devices. It stores
/// recently accessed blocks in memory to reduce the number of physical
/// device accesses.
///
/// # Fields
/// * `device` - The underlying block device being cached
/// * `cache` - Thread-safe cache storage using a B-tree map
pub struct BlockCache {
    device: Arc<dyn BlockDevice>,
    cache: RwLock<BTreeMap<usize, Vec<u8>>>,
}

impl BlockCache {
    /// Creates a new block cache for the given device
    ///
    /// # Arguments
    /// * `device` - The block device to cache
    ///
    /// # Returns
    /// A new `BlockCache` instance
    ///
    /// # Examples
    /// ```rust
    /// use crate::fs::bio::BlockCache;
    /// use crate::drivers::device::BlockDevice;
    /// use alloc::sync::Arc;
    ///
    /// let device: Arc<dyn BlockDevice> = /* get block device */;
    /// let cache = BlockCache::new(device);
    /// ```
    pub fn new(device: Arc<dyn BlockDevice>) -> Self {
        Self {
            device,
            cache: RwLock::new(BTreeMap::new()),
        }
    }

    /// Reads a block from the cache or device
    ///
    /// This method first checks if the block is in the cache. If it is,
    /// returns the cached data. If not, reads from the underlying device,
    /// stores it in the cache, and returns the data.
    ///
    /// # Arguments
    /// * `block_id` - The block number to read
    ///
    /// # Returns
    /// * `Some(Vec<u8>)` - The block data if successful
    /// * `None` - If the read from the device failed
    ///
    /// # Examples
    /// ```rust
    /// use crate::fs::bio::BlockCache;
    ///
    /// let cache = /* get cache instance */;
    /// if let Some(data) = cache.read_block(0) {
    ///     println!("Read block 0: {:?}", &data[..16]);
    /// }
    /// ```
    pub fn read_block(&self, block_id: usize) -> Option<Vec<u8>> {
        {
            let cache = self.cache.read();
            if let Some(data) = cache.get(&block_id) {
                return Some(data.clone());
            }
        }

        let block_size = self.device.block_size();
        let mut buf = Vec::with_capacity(block_size);
        buf.resize(block_size, 0);

        if self.device.read_blocks(block_id, 1, &mut buf).is_ok() {
            let mut cache = self.cache.write();
            cache.insert(block_id, buf.clone());
            Some(buf)
        } else {
            None
        }
    }

    /// Writes a block to both cache and device
    ///
    /// This method implements write-through caching: the data is written
    /// to both the cache and the underlying device. If the device write
    /// fails, the cache is not updated.
    ///
    /// # Arguments
    /// * `block_id` - The block number to write
    /// * `data` - The data to write (must match block size)
    ///
    /// # Returns
    /// * `true` - If the write succeeded
    /// * `false` - If the write failed
    ///
    /// # Panics
    /// This method does not panic, but returns `false` on failure.
    ///
    /// # Examples
    /// ```rust
    /// use crate::fs::bio::BlockCache;
    ///
    /// let cache = /* get cache instance */;
    /// let data = vec![0u8; 512]; // Assuming 512-byte blocks
    /// if cache.write_block(0, &data) {
    ///     println!("Successfully wrote block 0");
    /// }
    /// ```
    pub fn write_block(&self, block_id: usize, data: &[u8]) -> bool {
        if self.device.write_blocks(block_id, 1, data).is_ok() {
            let mut cache = self.cache.write();
            cache.insert(block_id, data.to_vec());
            true
        } else {
            false
        }
    }

    /// Synchronizes the cache with the device
    ///
    /// This is currently a no-op as the cache uses write-through
    /// semantics. All writes are immediately written to the device,
    /// so there's no need to flush dirty blocks.
    ///
    /// # Future Implementation
    /// When write-back caching is implemented, this method will
    /// flush all dirty blocks to the device.
    ///
    /// # Examples
    /// ```rust
    /// use crate::fs::bio::BlockCache;
    ///
    /// let cache = /* get cache instance */;
    /// cache.sync(); // Currently does nothing
    /// ```
    pub fn sync(&self) {}
}

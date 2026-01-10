//! Block device drivers
//!
//! This module provides block device driver implementations for storage devices.
//!
//! ## Overview
//!
//! Block devices are storage devices that read and write data in fixed-size blocks.
//! This module includes:
//! - Block device trait definitions
//! - RAM-based block device implementation
//! - Future implementations for disk, SSD, and other storage devices
//!
//! ## Submodules
//!
//! - [`ramblk`] - RAM-based block device implementation
//!
//! ## Usage
//!
//! ```rust
//! use proka_kernel::drivers::block::*;
//! use proka_kernel::drivers::DeviceInner;
//!
//! // Create a RAM block device
//! let ram_device = ramblk::RamBlockDevice::new(1024 * 1024); // 1MB RAM disk
//! let device = Device::new(
//!     "ramdisk".to_string(),
//!     2, // major number for block devices
//!     0, // minor number
//!     DeviceInner::Block(Arc::new(ram_device))
//! );
//!
//! // Register and use the device
//! if let Ok(registered) = DEVICE_MANAGER.write().register_device(device) {
//!     if let Some(block_dev) = registered.as_block_device() {
//!         let mut buffer = [0u8; 512];
//!         block_dev.read_blocks(0, 1, &mut buffer).expect("Read failed");
//!     }
//! }
//! ```
//!
//! ## Safety
//!
//! Block device operations involve direct memory access and may require proper
//! synchronization when accessed from multiple threads or interrupt contexts.
//!
//! ## Examples
//!
//! See the [`ramblk`] module for specific examples of RAM block device usage.

pub mod ramblk;

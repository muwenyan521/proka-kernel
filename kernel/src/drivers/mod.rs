//! Device drivers module
//!
//! This module provides device driver infrastructure and implementations for various
//! hardware components in the Proka kernel.
//!
//! ## Overview
//!
//! The drivers module is responsible for:
//! - Device discovery and management
//! - Abstract device interfaces (block, character, input)
//! - Driver registration and lifecycle management
//! - Device I/O operations
//!
//! ## Submodules
//!
//! - [`block`] - Block device drivers (disk, storage)
//! - [`mod@char`] - Character device drivers (serial, terminal)
//! - [`device`] - Core device management and abstractions
//! - [`input`] - Input device drivers (keyboard, mouse)
//!
//! ## Usage
//!
//! ```rust
//! use proka_kernel::drivers::*;
//!
//! // Register a device
//! let device = Device::new("my_device".to_string(), 1, 0, DeviceInner::Char(arc_device));
//! let registered = DEVICE_MANAGER.write().register_device(device);
//!
//! // Use device operations
//! if let Ok(device) = registered {
//!     device.open().expect("Failed to open device");
//!     // ... perform I/O operations
//!     device.close().expect("Failed to close device");
//! }
//! ```
//!
//! ## Safety
//!
//! Device drivers often interact directly with hardware and may contain unsafe code.
//! Proper synchronization and hardware abstraction must be maintained to ensure
//! system stability.
//!
//! ## Examples
//!
//! See individual submodule documentation for driver-specific examples.

pub mod block;
pub mod char;
pub mod device;
pub mod input;

pub use device::*;

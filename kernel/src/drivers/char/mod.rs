//! Character device drivers
//!
//! This module provides character device driver implementations for serial communication
//! and other character-oriented devices.
//!
//! ## Overview
//!
//! Character devices are devices that transfer data as a stream of bytes (characters).
//! This module includes:
//! - Character device trait definitions
//! - Serial port (UART) driver implementation
//! - Future implementations for terminals, consoles, and other character devices
//!
//! ## Submodules
//!
//! - [`serial`] - Serial port (UART) driver implementation
//!
//! ## Usage
//!
//! ```rust
//! use proka_kernel::drivers::char::*;
//! use proka_kernel::drivers::DeviceInner;
//!
//! // Create a serial device
//! let serial_device = serial::SerialDevice::new(1, 0, 0x3f8); // COM1 at 0x3f8
//! let device = Device::new(
//!     "serial0".to_string(),
//!     1, // major number for character devices
//!     0, // minor number
//!     DeviceInner::Char(Arc::new(serial_device))
//! );
//!
//! // Register and use the device
//! if let Ok(registered) = DEVICE_MANAGER.write().register_device(device) {
//!     if let Some(char_dev) = registered.as_char_device() {
//!         let mut buffer = [0u8; 32];
//!         char_dev.write(b"Hello, World!\n").expect("Write failed");
//!         let bytes_read = char_dev.read(&mut buffer).expect("Read failed");
//!     }
//! }
//! ```
//!
//! ## Safety
//!
//! Character device operations involve direct hardware access and may require
//! proper synchronization when accessed from multiple threads or interrupt contexts.
//! Serial port access in particular requires careful timing and interrupt handling.
//!
//! ## Examples
//!
//! See the [`serial`] module for specific examples of serial port usage.

pub mod serial;

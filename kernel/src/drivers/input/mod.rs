//! Input device drivers
//!
//! This module provides input device driver implementations for user input devices
//! such as keyboards, mice, and other human interface devices.
//!
//! ## Overview
//!
//! Input devices allow users to interact with the system. This module includes:
//! - Input device trait definitions
//! - Keyboard driver implementation
//! - Future implementations for mice, touchscreens, and other input devices
//!
//! ## Submodules
//!
//! - [`keyboard`] - Keyboard driver implementation
//!
//! ## Usage
//!
//! ```rust
//! use proka_kernel::drivers::input::*;
//! use proka_kernel::drivers::DeviceInner;
//!
//! // Create a keyboard device
//! let keyboard_device = keyboard::Keyboard::new();
//! let device = Device::new(
//!     "keyboard0".to_string(),
//!     3, // major number for input devices
//!     0, // minor number
//!     DeviceInner::Input(Arc::new(keyboard_device))
//! );
//!
//! // Note: The actual implementation may use Char or a different DeviceInner variant
//! // depending on how input devices are modeled in the system.
//! ```
//!
//! ## Safety
//!
//! Input device drivers interact with hardware interrupts and may require
//! careful handling of interrupt contexts and synchronization with user input
//! processing code.
//!
//! ## Examples
//!
//! See the [`keyboard`] module for specific examples of keyboard input handling.

pub mod keyboard;

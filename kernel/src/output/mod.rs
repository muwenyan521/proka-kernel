//! Output module for the Proka kernel
//!
//! This module provides output facilities for the kernel, supporting multiple
//! output destinations including serial ports, console display, and combined
//! output streams.
//!
//! # Submodules
//! - [`console`]: Console output for text display on screen
//! - [`dual`]: Dual output that writes to multiple destinations simultaneously
//! - [`serial`]: Serial port output for debugging and external communication
//!
//! # Output Architecture
//! The output system provides a flexible, layered approach:
//! 1. **Low-level Drivers**: Direct hardware access (serial ports, framebuffer)
//! 2. **Output Implementations**: Specific output methods (console, serial)
//! 3. **Combined Output**: Multi-destination output via the dual module
//!
//! # Features
//! - **Multiple Output Destinations**: Support for screen, serial, and combined output
//! - **Formatted Output**: `core::fmt` support for all output types
//! - **Thread Safety**: Safe concurrent access to output devices
//! - **Buffering**: Optional buffering for performance
//! - **Logging Integration**: Seamless integration with kernel logging system
//!
//! # Examples
//! ```rust
//! use crate::output::{console, serial, dual};
//! use core::fmt::Write;
//!
//! // Initialize console output
//! console::init();
//! 
//! // Write to console
//! writeln!(console::CONSOLE.lock(), "Hello, console!").unwrap();
//! 
//! // Initialize serial output
//! serial::init();
//! 
//! // Write to serial port
//! writeln!(serial::COM1.lock(), "Debug message").unwrap();
//! 
//! // Create dual output (both console and serial)
//! let mut dual_output = dual::DualOutput::new(
//!     console::CONSOLE.lock(),
//!     serial::COM1.lock()
//! );
//! writeln!(dual_output, "Message to both destinations").unwrap();
//! ```
//!
//! # Safety
//! This module contains unsafe code for:
//! - Direct hardware access to serial ports and display controllers
//! - Memory-mapped I/O operations for output devices
//! - Raw pointer manipulation for framebuffer access
//!
//! All output operations ensure proper device initialization and
//! synchronization to prevent data corruption.

pub mod console;
pub mod dual;
pub mod serial;

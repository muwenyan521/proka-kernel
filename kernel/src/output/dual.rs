//! Dual output module for simultaneous console and serial output
//!
//! This module provides macros and functions for outputting text to both the console
//! (graphical display) and serial port simultaneously. This is particularly useful
//! for debugging purposes, as it allows developers to see output both on-screen
//! and in a serial terminal.
//!
//! # Macros
//! - [`dual_print!`](macro.dual_print.html): Outputs formatted text to both console and serial
//! - [`dual_println!`](macro.dual_println.html): Outputs formatted text with newline to both console and serial
//!
//! # Internal Functions
//! - [`_dual_print_console`]: Internal function for console output
//! - [`_dual_print_serial`]: Internal function for serial output
//!
//! # Examples
//! ```rust
//! use crate::dual_print;
//! use crate::dual_println;
//!
//! // Print without newline
//! dual_print!("Hello, ");
//! dual_print!("World!");
//!
//! // Print with newline
//! dual_println!("Debug message: {}", 42);
//! ```
//!
//! # Usage
//! The dual output system is typically used for kernel debugging where you want to
//! see output both on the screen (for immediate feedback) and in a serial console
//! (for logging and remote debugging).
//!
//! # Note
//! This module re-exports the macros using `#[macro_export]`, making them available
//! at the crate root level as `dual_print!` and `dual_println!`.

use crate::output::console::_print as console_print;
use crate::output::serial::_print as serial_print;

/// Dual print macro: outputs to both console and serial simultaneously
///
/// This macro formats its arguments using the standard Rust formatting syntax
/// and sends the output to both the graphical console and serial port.
///
/// # Arguments
/// * `$($arg:tt)*` - Format string and arguments using Rust's formatting syntax
///
/// # Behavior
/// 1. First sends output to serial port via [`_dual_print_serial`]
/// 2. Then sends output to console via [`_dual_print_console`]
///
/// # Examples
/// ```rust
/// use crate::dual_print;
///
/// // Basic usage
/// dual_print!("Hello, World!");
///
/// // With formatting
/// dual_print!("The answer is {}", 42);
///
/// // Multiple arguments
/// dual_print!("x = {}, y = {}", 10, 20);
/// ```
///
/// # Note
/// The output order is serial first, then console. This ensures that if console
/// output fails or hangs, the serial output (which is often used for debugging)
/// still receives the message.
#[macro_export]
macro_rules! dual_print {
    ($($arg:tt)*) => {
        {
            $crate::output::dual::_dual_print_serial(format_args!($($arg)*));
            // 总是输出到控制台
            $crate::output::dual::_dual_print_console(format_args!($($arg)*))
        }
    };
}

/// Dual print macro with newline: outputs to both console and serial with newline
///
/// This macro works like [`dual_print!`](macro.dual_print.html) but automatically appends a newline
/// character to the output.
///
/// # Arguments
/// * `$($arg:tt)*` - Format string and arguments using Rust's formatting syntax
///
/// # Behavior
/// - When called without arguments: outputs just a newline character
/// - When called with arguments: formats the arguments and appends a newline
///
/// # Examples
/// ```rust
/// use crate::dual_println;
///
/// // Empty line
/// dual_println!();
///
/// // With message
/// dual_println!("Hello, World!");
///
/// // With formatting
/// dual_println!("Debug: x={}, y={}", 10, 20);
/// ```
///
/// # Note
/// This is the preferred macro for most debugging output as it ensures each
/// message is on its own line, making logs easier to read.
#[macro_export]
macro_rules! dual_println {
    () => {
        $crate::dual_print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::dual_print!("{}\n", format_args!($($arg)*))
    };
}

/// Internal function: handles console output for dual printing
///
/// This function is marked as `#[doc(hidden)]` because it's an implementation
/// detail used by the [`dual_print!`] and [`dual_println!`] macros. It forwards
/// the formatted arguments to the console output system.
///
/// # Arguments
/// * `args` - Formatted arguments to output to the console
///
/// # Behavior
/// Calls [`console_print`] from the console module to handle the actual output.
///
/// # Safety
/// This function is safe to call as it delegates to the safe console printing
/// function.
#[doc(hidden)]
pub fn _dual_print_console(args: core::fmt::Arguments) {
    console_print(args);
}

/// Internal function: handles serial output for dual printing
///
/// This function is marked as `#[doc(hidden)]` because it's an implementation
/// detail used by the [`dual_print!`] and [`dual_println!`] macros. It forwards
/// the formatted arguments to the serial output system.
///
/// # Arguments
/// * `args` - Formatted arguments to output to the serial port
///
/// # Behavior
/// Calls [`serial_print`] from the serial module to handle the actual output.
///
/// # Safety
/// This function is safe to call as it delegates to the safe serial printing
/// function.
#[doc(hidden)]
pub fn _dual_print_serial(args: core::fmt::Arguments) {
    serial_print(args);
}

//! Panic handling for the Proka kernel.
//!
//! This module provides panic handlers for both normal kernel operation and testing environments.
//! When a panic occurs, the handler prints the panic information to the serial console and
//! enters an infinite loop to halt the system.
//!
//! # Overview
//!
//! The module contains two main panic handlers:
//! 1. `panic` - The default panic handler for normal kernel operation
//! 2. `panic_for_test` - A specialized panic handler for test environments
//!
//! # Configuration
//!
//! The appropriate panic handler is selected at compile time based on the build configuration:
//! - Normal builds use the default panic handler
//! - Test builds use the test-specific panic handler
//!
//! # Safety
//!
//! Panic handlers are marked as `#[panic_handler]` and must never return. They are required
//! to be `unsafe` in the sense that they handle unrecoverable system states.
//!
//! # Examples
//!
//! When a panic occurs during normal kernel operation:
//! ```no_run
//! // This would trigger the panic handler
//! panic!("Something went wrong");
//! ```
//!
//! During testing, panics are handled differently to provide better test output:
//! ```no_run
//! #[test]
//! fn test_panic() {
//!     panic!("Test failure");
//! }
//! ```

use crate::serial_println;
use core::panic::PanicInfo;

/// The default panic handler for normal kernel operation.
///
/// This function is called when a panic occurs during normal kernel execution.
/// It prints the panic information to the serial console and enters an infinite loop
/// to halt the system.
///
/// # Arguments
/// * `info` - Information about the panic, including the panic message and location
///
/// # Returns
/// This function never returns (`-> !`).
///
/// # Behavior
/// 1. Prints the panic information to the serial console using `serial_println!`
/// 2. Enters an infinite loop to prevent further execution
///
/// # Safety
/// This function is marked as `#[panic_handler]` and must never return. It handles
/// unrecoverable system states.
#[cfg(not(test))]
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    serial_println!("{}", info);
    loop {}
}

/// Panic handler for test environments.
///
/// This function is called when a panic occurs during test execution.
/// It delegates to the `panic_for_test` function to provide test-specific
/// panic handling.
///
/// # Arguments
/// * `info` - Information about the panic
///
/// # Returns
/// This function never returns (`-> !`).
///
/// # See Also
/// [`panic_for_test`] - The actual test panic handler implementation
#[cfg(test)]
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    // Call the default test panic handler
    panic_for_test(info)
}

/// Test-specific panic handler implementation.
///
/// This function provides enhanced panic output for test failures, including
/// formatted error messages and test failure indicators.
///
/// # Arguments
/// * `info` - Information about the panic
///
/// # Returns
/// This function never returns (`-> !`).
///
/// # Behavior
/// 1. Prints "failed" to indicate test failure
/// 2. Prints the panic cause with indentation for better readability
/// 3. Enters an infinite loop (in a real test environment, this would exit QEMU)
///
/// # Notes
/// In a complete test environment, this function would exit QEMU with a failure
/// exit code. The current implementation uses an infinite loop as a placeholder.
#[cfg(test)]
pub fn panic_for_test(info: &PanicInfo) -> ! {
    serial_println!("failed");
    serial_println!("Caused by:\n\t{}", info);
    // exit_qemu(QemuExitCode::Failed);
    loop {} // Unreachable, but must write this
}

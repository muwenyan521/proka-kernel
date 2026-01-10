//! Test framework for the Proka kernel.
//!
//! This module provides a custom test framework for kernel testing, including
//! a test runner, testable trait, and QEMU integration for automated testing.
//! The framework is designed to run tests in a kernel environment and report
//! results via the serial console.
//!
//! # Overview
//!
//! The test framework consists of:
//! 1. [`Testable`] trait - Defines the interface for test functions
//! 2. [`test_runner`] function - Executes a collection of tests
//! 3. [`QemuExitCode`] enum - Exit codes for QEMU integration
//! 4. [`exit_qemu`] function - Exits QEMU with a specified exit code
//! 5. [`kernel_main`](fn.kernel_main.html) function - Test entry point (test builds only)
//!
//! # Usage
//!
//! Tests are defined using the standard `#[test]` attribute. The test runner
//! automatically discovers and executes all tests, reporting results to the
//! serial console.
//!
//! # Examples
//!
//! ```rust
//! #[test]
//! fn test_example() {
//!     assert_eq!(2 + 2, 4);
//! }
//! ```
//!
//! # QEMU Integration
//!
//! When running tests in QEMU, the test framework uses port 0xf4 to communicate
//! exit codes to the host system. This allows automated test runners to
//! determine whether tests passed or failed.
//!
//! # Safety
//!
//! The `exit_qemu` function uses unsafe code to write to I/O ports. This is
//! necessary for QEMU integration but requires careful handling.

use crate::{serial_print, serial_println};

/// A trait for testable functions.
///
/// This trait defines the interface that all test functions must implement.
/// It provides a standardized way to execute tests and report their results.
///
/// # Implementation
///
/// The trait is automatically implemented for any type that implements `Fn()`.
/// This allows regular functions to be used as tests without additional boilerplate.
///
/// # Examples
///
/// ```rust
/// use crate::test::Testable;
///
/// fn my_test() {
///     assert_eq!(1 + 1, 2);
/// }
///
/// // my_test automatically implements Testable
/// let test: &dyn Testable = &my_test;
/// test.run();
/// ```
pub trait Testable {
    /// Executes the test.
    ///
    /// This method runs the test function and reports the result.
    /// Implementations should handle any panics and provide appropriate
    /// output to indicate success or failure.
    fn run(&self);
}

/// Default implementation of `Testable` for any function type.
///
/// This implementation provides a standard test execution pattern:
/// 1. Prints the test name
/// 2. Executes the test function
/// 3. Prints `[ok]` if the test completes successfully
///
/// If the test panics, the panic handler will catch it and report failure.
impl<T> Testable for T
where
    T: Fn(),
{
    /// Executes the test function with standard reporting.
    ///
    /// # Behavior
    /// 1. Prints the test name using `core::any::type_name::<T>()`
    /// 2. Executes the test function
    /// 3. Prints `[ok]` if execution completes without panicking
    ///
    /// # Panics
    /// If the test function panics, the panic will propagate to the panic handler.
    fn run(&self) {
        serial_print!("Testing {}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

/// The main test runner function.
///
/// This function executes all provided tests and reports the results.
/// After running all tests, it exits QEMU with a success code.
///
/// # Arguments
/// * `tests` - A slice of test references to execute
///
/// # Behavior
/// 1. Prints the total number of tests
/// 2. Executes each test in order using its `run()` method
/// 3. Exits QEMU with `QemuExitCode::Success`
///
/// # Panics
/// If any test panics, the panic handler will be invoked and QEMU will
/// exit with a failure code.
///
/// # Examples
/// ```rust
/// use crate::test::{test_runner, Testable};
///
/// fn test1() { /* ... */ }
/// fn test2() { /* ... */ }
///
/// #[test_case]
/// fn run_all_tests() {
///     let tests: &[&dyn Testable] = &[&test1, &test2];
///     test_runner(tests);
/// }
/// ```
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

/// Exit codes for QEMU integration.
///
/// These codes are written to port 0xf4 to communicate test results
/// to the host system. The host can check the exit code to determine
/// whether tests passed or failed.
///
/// # Values
/// - `Success = 0x10` - All tests passed
/// - `Failed = 0x11` - One or more tests failed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QemuExitCode {
    /// All tests passed successfully.
    Success = 0x10,
    /// One or more tests failed.
    Failed = 0x11,
}

/// Exits QEMU with the specified exit code.
///
/// This function writes the exit code to port 0xf4, which is monitored
/// by QEMU. The host system can check this port to determine the
/// test execution result.
///
/// # Arguments
/// * `exit_code` - The exit code to send to QEMU
///
/// # Safety
/// This function uses unsafe code to write to I/O port 0xf4.
/// The port write is a privileged operation that requires careful handling.
///
/// # Behavior
/// Writes the exit code as a 32-bit value to port 0xf4, causing QEMU to exit.
///
/// # Examples
/// ```rust
/// use crate::test::{exit_qemu, QemuExitCode};
///
/// // Exit QEMU with success code
/// exit_qemu(QemuExitCode::Success);
///
/// // Exit QEMU with failure code
/// exit_qemu(QemuExitCode::Failed);
/// ```
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

/// Test entry point for kernel test builds.
///
/// This function serves as the entry point when the kernel is compiled
/// for testing. It calls the main test function and then enters an
/// infinite loop.
///
/// # Behavior
/// 1. Calls `crate::test_main()` to execute tests
/// 2. Enters an infinite loop (should never be reached if tests exit QEMU)
///
/// # Safety
/// This function is marked with `#[unsafe(no_mangle)]` to preserve its
/// name during linking. It's the entry point for test builds.
///
/// # Notes
/// In a properly functioning test environment, `test_main()` should call
/// `exit_qemu()` to exit QEMU, so the infinite loop should never be reached.
#[cfg(test)]
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    crate::test_main();
    loop {}
}

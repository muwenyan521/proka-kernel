//! # Proka Kernel - A kernel for ProkaOS
//!
//! This is the main library crate for the Proka kernel, providing the core functionality
//! for ProkaOS. The kernel is designed to be modular, extensible, and follows modern
//! operating system design principles.
//!
//! Copyright (C) RainSTR Studio 2025, All rights reserved.
//!
//! ## Overview
//!
//! The Proka kernel provides:
//! - Memory management (paging, frame allocation, heap allocation)
//! - Device drivers (block, character, input devices)
//! - File system support (VFS, in-memory FS, kernel FS)
//! - Graphics and display output
//! - Interrupt handling (APIC, PIC, IDT, GDT)
//! - System libraries (logger, BMP parser, initrd, MSR access)
//! - Output mechanisms (console, serial, dual output)
//!
//! ## Modules
//!
//! - [`drivers`] - Device drivers for various hardware components
//! - [`fs`] - File system implementations and virtual file system
//! - [`graphics`] - Graphics and display functionality
//! - [`interrupts`] - Interrupt handling and processor control
//! - [`libs`] - Utility libraries and helpers
//! - [`memory`] - Memory management and allocation
//! - [`output`] - Output mechanisms for kernel messages
//! - [`mod@panic`] - Panic handling and reporting
//! - [`mod@test`] - Testing framework and utilities
//!
//! ## Usage
//!
//! To use this kernel in your OS project:
//!
//! ```rust
//! use proka_kernel::*;
//!
//! // Initialize kernel subsystems
//! init_frame_allocator();
//! init_offset_page_table();
//! // ... other initialization
//! ```
//!
//! ## Safety
//!
//! This kernel uses `#![no_std]` and contains unsafe code for low-level operations.
//! Proper synchronization and memory safety must be ensured by the caller when
//! using unsafe APIs.
//!
//! ## Examples
//!
//! See the individual module documentation for specific examples of using each
//! kernel subsystem.

#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]
pub mod drivers;
pub mod fs;
pub mod graphics;
pub mod interrupts;
pub mod libs;
pub mod memory;
pub mod output;
pub mod panic;
pub mod test;

// Re-export common memory management types and functions
pub use memory::frame_allocator::{format_bytes, FrameStats, LockedFrameAllocator};
pub use memory::paging::{
    get_hhdm_offset, get_memory_stats, init_frame_allocator, init_offset_page_table,
    print_memory_stats,
};
pub use memory::protection::{kernel_flags, user_flags, Protection};

use limine::{
    modules::InternalModule,
    request::{FramebufferRequest, MemoryMapRequest, ModuleRequest},
    BaseRevision,
};

/* The section data define area */
#[unsafe(link_section = ".requests")]
#[used]
/// The base revision of the kernel.
pub static BASE_REVISION: BaseRevision = BaseRevision::new();

#[unsafe(link_section = ".requests")]
#[used]
/// The framebuffer request of the kernel.
pub static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[unsafe(link_section = ".requests")]
#[used]
pub static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[unsafe(link_section = ".requests")]
#[used]
pub static HHDM_REQUEST: limine::request::HhdmRequest = limine::request::HhdmRequest::new();

#[unsafe(link_section = ".requests")]
#[used]
pub static MODULE_REQUEST: ModuleRequest = ModuleRequest::new()
    .with_internal_modules(&[&InternalModule::new().with_path(c"/initrd.cpio")]);

/// This will extern the C function and make it to safe.
///
/// # Example
/// ```rust
/// use proka_kernel::extern_safe;
///
/// // Make sure that the C function was defined and linked currectly.
/// extern_safe! {
///     fn add(a: i32, b: i32) -> i32;
/// }
///
/// // Then use it, with the header "safe_".
/// let result = safe_add(1, 2);
/// assert_eq!(result, 3);
/// ```
#[macro_export]
macro_rules! extern_safe {
    (
        $(
            $(#[$meta:meta])*
            fn $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty;
        )*
    ) => {
        unsafe extern "C" {
            $(
                $(#[$meta])*
                fn $name($($arg: $ty),*) -> $ret;
            )*
        }

       $(
           paste::paste! {
                pub fn [<safe_ $name>]($($arg: $ty),*) -> $ret {
                    unsafe { $name($($arg),*) }
                }
            }
        )*
    };
}

/* The test functions */

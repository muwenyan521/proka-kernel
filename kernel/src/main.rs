//! Main entry point for the Proka kernel.
//!
//! This module contains the kernel's main entry point and initialization sequence.
//! It orchestrates the bootstrapping of all kernel subsystems and enters the
//! main execution loop.
//!
//! # Overview
//!
//! The kernel follows a structured initialization sequence:
//! 1. **Bootloader Verification** - Checks Limine bootloader compatibility
//! 2. **Memory Management** - Initializes paging, frame allocation, and heap
//! 3. **Device Initialization** - Sets up device drivers and device manager
//! 4. **Logging System** - Initializes the kernel logger
//! 5. **Interrupt Handling** - Sets up GDT, IDT, and interrupt controllers (PIC/APIC)
//! 6. **File System** - Loads initial RAM disk (initrd)
//! 7. **Main Loop** - Enters the primary execution loop with keyboard input handling
//!
//! # Architecture
//!
//! The kernel is designed for x86_64 systems and uses:
//! - No standard library (`#![no_std]`)
//! - Custom memory management with paging
//! - Modular driver architecture
//! - Virtual file system abstraction
//! - Dual interrupt controller support (PIC and APIC)
//!
//! # Safety
//!
//! This module contains unsafe operations for:
//! - Direct hardware access (memory mapping, I/O ports)
//! - Interrupt controller configuration
//! - Low-level system initialization
//!
//! All unsafe operations are carefully documented and isolated.
//!
//! # Examples
//!
//! The kernel is typically booted via a bootloader like Limine. Once loaded,
//! it initializes all subsystems and provides a basic interactive environment
//! with keyboard input and file system access.
//!
//! For more information, see the [Proka Kernel GitHub repository](https://github.com/RainSTR-Studio/proka-kernel).

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(proka_kernel::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

/* Module imports */
#[macro_use]
extern crate proka_kernel;
extern crate alloc;
use log::info;
use proka_kernel::drivers::init_devices;
use proka_kernel::fs::vfs::VFS;
use proka_kernel::BASE_REVISION;

/* C functions extern area */
extern_safe! {
    /// Adds two 32-bit integers.
    ///
    /// # Arguments
    /// * `a` - First integer
    /// * `b` - Second integer
    ///
    /// # Returns
    /// The sum of `a` and `b`
    fn add(a: i32, b: i32) -> i32;
    
    /// Subtracts two 32-bit integers.
    ///
    /// # Arguments
    /// * `a` - First integer
    /// * `b` - Second integer
    ///
    /// # Returns
    /// The difference `a - b`
    fn sub(a: i32, b: i32) -> i32;
}

/// The main kernel entry point.
///
/// This function is called by the bootloader after the kernel is loaded.
/// It performs all system initialization and enters the main execution loop.
///
/// # Initialization Sequence
///
/// 1. **Bootloader Verification**: Checks Limine bootloader compatibility
/// 2. **Memory Management**:
///    - Retrieves memory map from bootloader
///    - Initializes paging with HHDM (Higher Half Direct Map) offset
///    - Sets up frame allocator
///    - Initializes kernel heap
/// 3. **Device System**: Initializes all device drivers
/// 4. **Logging**: Sets up the kernel logger
/// 5. **Console**: Hides the console cursor
/// 6. **File System**: Loads the initial RAM disk (initrd)
/// 7. **Interrupt Handling**:
///    - Initializes Global Descriptor Table (GDT)
///    - Sets up Interrupt Descriptor Table (IDT)
///    - Configures Programmable Interrupt Controller (PIC)
///    - Attempts to enable Advanced Programmable Interrupt Controller (APIC)
/// 8. **Device Enumeration**: Lists all detected devices
/// 9. **File Access**: Demonstrates file system access by reading a test file
/// 10. **Main Loop**: Enters keyboard input handling loop
///
/// # Returns
/// This function never returns (`-> !`).
///
/// # Safety
/// This function performs unsafe operations including:
/// - Direct memory mapping and page table manipulation
/// - Interrupt controller configuration
/// - Hardware I/O operations
///
/// All unsafe operations are carefully controlled and documented.
///
/// # Panics
/// The function will panic if:
/// - The Limine bootloader version is not supported
/// - Memory map cannot be retrieved from the bootloader
/// - Heap initialization fails
/// - File system operations fail (in demonstration code)
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    // Check if Limine version is supported
    assert!(BASE_REVISION.is_supported(), "Limine version not supported");

    // Initialize memory management
    let memory_map_response = proka_kernel::MEMORY_MAP_REQUEST
        .get_response()
        .expect("Failed to get memory map response");

    let hhdm_offset = proka_kernel::memory::paging::get_hhdm_offset();
    let mut mapper = unsafe { proka_kernel::memory::paging::init_offset_page_table(hhdm_offset) };
    let mut frame_allocator =
        unsafe { proka_kernel::memory::paging::init_frame_allocator(memory_map_response) };

    proka_kernel::memory::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("Failed to initialize heap");

    init_devices();
    proka_kernel::libs::logger::init_logger(); // Init log system

    proka_kernel::output::console::CONSOLE
        .lock()
        .cursor_hidden();

    info!("Heap initialized");
    info!("Paging initialized");

    // Print memory statistics
    proka_kernel::memory::paging::print_memory_stats(&frame_allocator);

    proka_kernel::libs::initrd::load_initrd();

    // Initialize various modules
    proka_kernel::interrupts::gdt::init();
    info!("GDT Initialized");
    proka_kernel::interrupts::idt::init_idt();
    info!("IDT initialized");

    // Initialize Interrupt Controller
    // We default to PIC for now as APIC support is partial (no IOAPIC yet)
    proka_kernel::interrupts::pic::init();
    info!("PIC initialized");

    // Try to enable APIC if available
    if proka_kernel::interrupts::apic::init() {
        info!("APIC detected and enabled");
    } else {
        info!("Using legacy PIC only");
    }

    x86_64::instructions::interrupts::enable();

    println!("Device list:");
    for device in proka_kernel::drivers::DEVICE_MANAGER
        .read()
        .list_devices()
        .iter()
    {
        println!("{:?}", device);
    }
    let fp = VFS.open("test.txt").expect("Can't open initrd");
    let mut buf = [0u8; 1024];
    let len = fp.read(&mut buf).expect("Failed to read file");
    println!(
        "File content: {}",
        core::str::from_utf8(&buf[..len]).unwrap()
    );

    // Main execution loop
    loop {
        let mut buf = [0u8; 1];
        let kbd_device = {
            let device_manager = proka_kernel::drivers::DEVICE_MANAGER.read();
            device_manager.get_device("keyboard")
        };

        if let Some(kbd_device) = kbd_device {
            if let Some(char_dev) = kbd_device.as_char_device() {
                if let Ok(count) = char_dev.read(&mut buf) {
                    if count > 0 {
                        print!("{}", buf[0] as char);
                    }
                }
            }
        }
        x86_64::instructions::hlt();
    }
}

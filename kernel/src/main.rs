// src/main.rs
//! Proka Kernel - A kernel for ProkaOS
//! Copyright (C) RainSTR Studio 2025, All Rights Reserved.
//!
//! Well, welcome to the main entry of Proka Kernel!!
//!
//! If you have jumped here successfully, that means your CPU
//! can satisfy our kernel's requirements.
//!
//! Now, let's enjoy the kernel written in Rust!!!!
//!
//! For more information, see https://github.com/RainSTR-Studio/proka-kernel

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
    fn add(a: i32, b: i32) -> i32;
    fn sub(a: i32, b: i32) -> i32;
}

/* The Kernel main code */
// The normal one
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    // Check is limine version supported
    assert!(BASE_REVISION.is_supported(), "Limine version not supported");

    // 初始化内存管理
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

    // 初始化各个模块
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

    loop {
        x86_64::instructions::hlt();
    }
}

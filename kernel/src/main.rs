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
use log::{debug, info};
use proka_kernel::BASE_REVISION;
use proka_kernel::drivers::init_devices;
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
    init_devices();
    proka_kernel::libs::logger::init_logger(); // Init log system

    proka_kernel::output::console::CONSOLE
        .lock()
        .cursor_hidden();

    println!("Starting ProkaOS v{}...", env!("CARGO_PKG_VERSION")); // Print welcome message

    // 初始化各个模块
    proka_kernel::interrupts::gdt::init();
    info!("GDT Initialized");
    proka_kernel::interrupts::idt::init_idt();
    info!("IDT initialized");
    proka_kernel::interrupts::apic::init();
    info!("APIC initialized");

    //proka_kernel::memory::paging::table::init_page_table();

    println!("Device list:");
    for device in proka_kernel::drivers::DEVICE_MANAGER
        .lock()
        .list_devices()
        .iter()
    {
        println!("{:?}", device);
    }

    success!("Kernel ready!");

    let vfs = proka_kernel::fs::vfs::Vfs::new();
    vfs.mount(None, "/", "memfs", None).unwrap();
    let root_node = vfs.lookup("/").unwrap();
    root_node
        .create("myfile.txt", proka_kernel::fs::vfs::VNodeType::File)
        .unwrap();
    let file_node = vfs.lookup("/myfile.txt").unwrap();

    {
        let mut file = file_node.open().unwrap();
        file.write(b"Hello, MemFs!").unwrap();
    }

    // Read file
    {
        let file = file_node.open().unwrap();
        let mut buf = [0u8; 32];
        let len = file.read(&mut buf).unwrap();
        println!(
            "Read content: {}",
            alloc::string::String::from_utf8_lossy(&buf[..len])
        );
    }

    loop {
        x86_64::instructions::hlt();
    }
}

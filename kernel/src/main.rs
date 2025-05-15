#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::arch::asm;
use multiboot2::{BootInformation, BootInformationHeader, MAGIC};
#[macro_use] extern crate proka_kernel;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    x86_64::instructions::interrupts::int3();
    loop {}
}

/* C functions extern area */
extern_safe! {
    fn add(a: i32, b: i32) -> i32;
    fn sub(a: i32, b: i32) -> i32;
}

/* The Kernel main code */
#[unsafe(no_mangle)]
pub fn kernel_main() -> ! {
    // Initialize the value
    let mb_magic: u32;
    let mbi_ptr: u32;

    // Get EAX and EBX value as the value
    unsafe {
        asm!("mov {0:e}, eax", out(reg) mb_magic);
        asm!("mov {0:e}, ebx", out(reg) mbi_ptr);
    }

    if mb_magic + 1 != MAGIC {
        panic!("The kernel does not support multiboot2.")
    }
    // Get the multiboot2 information
    let boot_info =
        unsafe { BootInformation::load(mbi_ptr as *const BootInformationHeader).expect("Bootloader parsing failed") };

    // Get the framebuffer tag.
    // In multiboot2 crate, the "info.framebuffer_tag()" will
    // return a Some(Ok(framebuffer)), so use "match" to handle.
    let framebuffer = match boot_info.framebuffer_tag() {
        Some(Ok(tag)) => tag,
        Some(Err(_)) => panic!("Unknown framebuffer type"),
        None => panic!("No framebuffer tag"),
    };

    loop {}
}

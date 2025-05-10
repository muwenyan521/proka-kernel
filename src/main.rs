#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

/* The Kernel main code */
#[unsafe(no_mangle)]
pub fn kernel_main() -> ! {
    loop {}
}

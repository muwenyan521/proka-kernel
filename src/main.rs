#![no_std]
#![no_main]

use core::panic::PanicInfo;
use proka_kernel::extern_safe;

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    unsafe {
        core::arch::asm!("int 3");
    }
    loop {}
}

/* The Kernel main code */
extern_safe! {
    fn add(a: i32, b: i32) -> i32;
    fn sub(a: i32, b: i32) -> i32;
}

#[unsafe(no_mangle)]
pub fn kernel_main() -> ! {
    let result = safe_add(3, 2);
    assert_eq!(result, 4);
    loop {}
}

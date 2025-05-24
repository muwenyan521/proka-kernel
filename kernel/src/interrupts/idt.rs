use crate::interrupts::error_handler;
use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;

// 使用 lazy_static 延迟初始化
lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        // 设置默认处理器中断
        idt.divide_error.set_handler_fn(error_handler::divide_error_handler);
        idt.breakpoint.set_handler_fn(error_handler::breakpoint_handler);
        idt
    };
}

// 初始化并加载IDT
pub fn init_idt() {
    IDT.load();
}

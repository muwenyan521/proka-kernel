use crate::interrupts::error_handler;
use crate::interrupts::gdt;
use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;

lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // 无错误码异常设置
        idt.divide_error.set_handler_fn(error_handler::divide_error_handler);
        idt.debug.set_handler_fn(error_handler::debug_handler);
        idt.non_maskable_interrupt.set_handler_fn(error_handler::nmi_handler);
        idt.breakpoint.set_handler_fn(error_handler::breakpoint_handler);
        idt.overflow.set_handler_fn(error_handler::overflow_handler);
        idt.bound_range_exceeded.set_handler_fn(error_handler::bound_range_handler);
        idt.invalid_opcode.set_handler_fn(error_handler::invalid_opcode_handler);
        idt.device_not_available.set_handler_fn(error_handler::device_not_available_handler);

        // 有错误码异常设置 (set_handler_fn 自动识别签名)
        idt.invalid_tss.set_handler_fn(error_handler::invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(error_handler::segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(error_handler::stack_segment_handler);
        idt.general_protection_fault.set_handler_fn(error_handler::general_protection_handler);
        idt.alignment_check.set_handler_fn(error_handler::alignment_check_handler);

        // 特殊异常设置
        idt.page_fault.set_handler_fn(error_handler::pagefault_handler);

        unsafe {
            idt.double_fault
                .set_handler_fn(error_handler::double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}

pub fn init_idt() {
    IDT.load();
    crate::serial_println!("Interrupt Descriptor Table loaded");
}

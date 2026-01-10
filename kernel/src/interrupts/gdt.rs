//! Global Descriptor Table (GDT) and Task State Segment (TSS) initialization
//!
//! This module provides functionality for setting up the Global Descriptor Table (GDT)
//! and Task State Segment (TSS) required for protected mode operation in x86_64 systems.
//!
//! # Overview
//! The GDT is a data structure used by x86_64 processors to define memory segments
//! and their attributes. The TSS is used for hardware task switching and interrupt
//! handling, particularly for providing separate stacks for exception handlers.
//!
//! # Key Components
//! - [`DOUBLE_FAULT_IST_INDEX`] - Interrupt Stack Table index for double fault handling
//! - [`init`] - Initialize GDT and TSS
//! - Static GDT and TSS instances created using `lazy_static!`
//!
//! # Safety
//! This module contains unsafe operations when loading segment selectors and TSS.
//! These operations must be performed at the appropriate time during kernel initialization.

use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

/// Interrupt Stack Table index for double fault handling
///
/// This constant defines which entry in the Interrupt Stack Table (IST)
/// should be used for double fault exceptions. Double faults require a
/// separate stack to prevent infinite recursion if a stack overflow occurs.
///
/// # Value
/// The value `0` indicates the first entry in the IST.
///
/// # See Also
/// - [x86_64 Task State Segment documentation](https://docs.rs/x86_64/latest/x86_64/structures/tss/struct.TaskStateSegment.html)
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 8192 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE as u64;
            stack_end
        };
        tss
    };
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let data_selector = gdt.append(Descriptor::kernel_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                data_selector,
                tss_selector,
            },
        )
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

/// Initialize the Global Descriptor Table (GDT) and Task State Segment (TSS)
///
/// This function performs the following operations:
/// 1. Loads the GDT into the CPU using the `lgdt` instruction
/// 2. Sets up segment registers (CS, SS, DS, ES) with appropriate selectors
/// 3. Loads the TSS selector into the CPU's Task Register
///
/// # Safety
/// This function contains unsafe operations because it directly modifies
/// CPU control registers. These operations must be performed during kernel
/// initialization and should not be called multiple times.
///
/// # Segment Registers
/// - `CS` (Code Segment): Set to kernel code selector
/// - `SS` (Stack Segment): Set to kernel data selector
/// - `DS` (Data Segment): Set to kernel data selector
/// - `ES` (Extra Segment): Set to kernel data selector
///
/// # TSS Loading
/// The TSS is loaded to provide a separate stack for double fault handling.
/// This prevents infinite recursion if a stack overflow causes a double fault.
///
/// # Panics
/// This function does not panic, but incorrect usage may cause undefined behavior.
///
/// # Examples
/// ```rust
/// use kernel::interrupts::gdt;
///
/// // Initialize GDT and TSS during kernel startup
/// gdt::init();
/// println!("GDT and TSS initialized");
/// ```
pub fn init() {
    use x86_64::instructions::segmentation::{Segment, CS, DS, ES, SS};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        SS::set_reg(GDT.1.data_selector);
        DS::set_reg(GDT.1.data_selector);
        ES::set_reg(GDT.1.data_selector);
        load_tss(GDT.1.tss_selector);
    }
}

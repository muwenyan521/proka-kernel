//! Interrupt handling module for the Proka kernel
//!
//! This module provides interrupt management facilities for the kernel, including
//! interrupt descriptor tables, interrupt controllers, and interrupt service routines.
//!
//! # Submodules
//! - [`apic`]: Advanced Programmable Interrupt Controller (APIC) support
//! - [`gdt`]: Global Descriptor Table management
//! - [`handler`]: Interrupt handler implementations
//! - [`idt`]: Interrupt Descriptor Table management
//! - [`pic`]: Programmable Interrupt Controller (PIC) support
//!
//! # Architecture
//! The interrupt system follows a layered approach:
//! 1. **Hardware Layer**: PIC/APIC controllers for hardware interrupt routing
//! 2. **Descriptor Tables**: GDT and IDT for defining interrupt gates and segments
//! 3. **Handler Layer**: Interrupt service routines for specific interrupt types
//!
//! # Interrupt Types
//! - **Exceptions**: CPU-generated interrupts (divide by zero, page fault, etc.)
//! - **Hardware Interrupts**: Device-generated interrupts (keyboard, timer, etc.)
//! - **Software Interrupts**: Programmatically triggered interrupts (system calls)
//!
//! # Examples
//! ```rust
//! use crate::interrupts::{idt, handler};
//!
//! // Initialize interrupt handling
//! idt::init();
//! 
//! // Register a custom interrupt handler
//! handler::register_handler(0x20, timer_handler);
//! 
//! // Enable interrupts
//! unsafe { asm!("sti") };
//! ```
//!
//! # Safety
//! This module contains extensive unsafe code for:
//! - Direct manipulation of CPU descriptor tables (GDT, IDT)
//! - Low-level interrupt controller programming (PIC, APIC)
//! - Assembly language interrupt handlers
//! - Memory-mapped I/O operations
//!
//! All interrupt-related operations must be performed with interrupts disabled
//! to prevent race conditions and ensure atomicity.

pub mod apic;
pub mod gdt;
pub mod handler;
pub mod idt;
pub mod pic;

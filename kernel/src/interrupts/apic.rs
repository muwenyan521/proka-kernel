//! Advanced Programmable Interrupt Controller (APIC) support
//!
//! This module provides functionality for detecting and configuring the APIC
//! (Advanced Programmable Interrupt Controller) and x2APIC extensions.
//!
//! APIC is a more advanced interrupt controller that replaces the legacy PIC
//! (Programmable Interrupt Controller) in modern x86 systems. It provides:
//! - Support for more interrupt vectors (up to 256)
//! - Symmetric multiprocessing (SMP) support
//! - Advanced power management features
//! - Message Signaled Interrupts (MSI)
//!
//! x2APIC is an extension that provides:
//! - 32-bit APIC IDs (vs 8-bit in standard APIC)
//! - MSI support without I/O space
//! - Better virtualization support
//!
//! # Functions
//! - [`apic_is_available`] - Check if APIC is available
//! - [`x2apic_is_available`] - Check if x2APIC extension is available
//! - [`enable_x2apic`] - Enable x2APIC mode
//! - [`init`] - Initialize APIC/x2APIC
//!
//! # Safety
//! This module contains unsafe operations when accessing MSRs (Model Specific Registers).
//! These operations require appropriate privileges and can cause system instability
//! if used incorrectly.

use crate::libs::msr;
use log::{debug, error};
use raw_cpuid::CpuId;
use x86_64::registers::model_specific::Msr;

/// Check if APIC (Advanced Programmable Interrupt Controller) is available
///
/// This function uses CPUID to check if the APIC feature is supported by the CPU.
/// APIC is required for symmetric multiprocessing (SMP) and advanced interrupt
/// handling in modern x86 systems.
///
/// # Returns
/// * `true` - APIC is available and can be used
/// * `false` - APIC is not available (legacy PIC must be used)
///
/// # Examples
/// ```rust
/// use kernel::interrupts::apic;
///
/// if apic::apic_is_available() {
///     println!("APIC is available");
/// } else {
///     println!("Using legacy PIC");
/// }
/// ```
pub fn apic_is_available() -> bool {
    let cpuid = CpuId::new();
    cpuid
        .get_feature_info()
        .map_or(false, |info| info.has_apic())
}

/// Check if x2APIC extension is available
///
/// x2APIC is an extension to the APIC architecture that provides:
/// - 32-bit APIC IDs (compared to 8-bit in standard APIC)
/// - MSI (Message Signaled Interrupts) without I/O space
/// - Better virtualization support
/// - Enhanced performance for large systems
///
/// This function uses CPUID to check if the x2APIC feature is supported.
/// x2APIC must be enabled before it can be used.
///
/// # Returns
/// * `true` - x2APIC extension is available
/// * `false` - x2APIC is not available (standard APIC must be used)
///
/// # Examples
/// ```rust
/// use kernel::interrupts::apic;
///
/// if apic::x2apic_is_available() {
///     println!("x2APIC is available");
/// } else {
///     println!("Only standard APIC is available");
/// }
/// ```
pub fn x2apic_is_available() -> bool {
    let cpuid = CpuId::new();
    cpuid
        .get_feature_info()
        .map_or(false, |info| info.has_x2apic())
}

/// Enable x2APIC mode
///
/// This function enables x2APIC mode by setting the x2APIC enable bit
/// in the IA32_APIC_BASE MSR (Model Specific Register).
///
/// # Safety
/// This function contains unsafe operations because it directly accesses
/// MSRs. Incorrect MSR access can cause system instability or crashes.
///
/// # Requirements
/// - x2APIC must be available (check with [`x2apic_is_available`])
/// - The CPU must be in an appropriate privilege level
/// - APIC must be available (check with [`apic_is_available`])
///
/// # Panics
/// This function does not panic, but incorrect usage may cause undefined behavior.
///
/// # Examples
/// ```rust
/// use kernel::interrupts::apic;
///
/// if apic::x2apic_is_available() {
///     // Safety: We've verified x2APIC is available
///     apic::enable_x2apic();
///     println!("x2APIC enabled");
/// }
/// ```
pub fn enable_x2apic() {
    unsafe {
        let mut apic_base = Msr::new(msr::IA32_APIC_BASE);
        let mut apic_base_raw = apic_base.read();
        apic_base_raw |= 1 << 10;
        apic_base.write(apic_base_raw);
    }
}

/// Initialize APIC or x2APIC
///
/// This function initializes the interrupt controller by:
/// 1. Checking if APIC is available
/// 2. If APIC is available, checking if x2APIC is available
/// 3. Enabling x2APIC if available, otherwise using standard APIC
///
/// The function logs appropriate debug messages and returns a boolean
/// indicating whether APIC/x2APIC was successfully initialized.
///
/// # Returns
/// * `true` - APIC or x2APIC was successfully initialized
/// * `false` - APIC is not supported (legacy PIC must be used)
///
/// # Logging
/// - Debug: "x2APIC is available" if x2APIC is enabled
/// - Debug: "APIC is available" if only standard APIC is available
/// - Error: "APIC not supported!" if APIC is not available
///
/// # Examples
/// ```rust
/// use kernel::interrupts::apic;
///
/// if apic::init() {
///     println!("APIC/x2APIC initialized successfully");
/// } else {
///     println!("Failed to initialize APIC, using legacy PIC");
/// }
/// ```
pub fn init() -> bool {
    if apic_is_available() {
        if x2apic_is_available() {
            debug!("x2APIC is available");
            enable_x2apic();
            return true;
        } else {
            debug!("APIC is available");
            return true;
        }
    } else {
        error!("APIC not supported!");
        return false;
    }
}

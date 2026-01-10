//! Utility libraries module for the Proka kernel
//!
//! This module provides various utility libraries and helper functions that are
//! used throughout the kernel for common tasks such as logging, file format
//! parsing, and hardware-specific operations.
//!
//! # Submodules
//! - [`bmp`]: BMP (Bitmap) image format parsing and manipulation
//! - [`initrd`]: Initial RAM disk (initrd) loading and management
//! - [`logger`]: Kernel logging facility with different log levels
//! - [`msr`]: Model-Specific Register (MSR) access and manipulation
//!
//! # Features
//! - **Image Processing**: BMP format support for kernel graphics
//! - **Boot Support**: Initrd handling for early filesystem access
//! - **Debugging**: Flexible logging system for kernel diagnostics
//! - **Hardware Control**: Low-level CPU register access via MSRs
//!
//! # Examples
//! ```rust
//! use crate::libs::{logger, bmp};
//!
//! // Initialize the kernel logger
//! logger::init();
//! 
//! // Log a message
//! log::info!("Kernel initialized successfully");
//! 
//! // Load a BMP image from memory
//! let bmp_data = include_bytes!("../../assets/logo.bmp");
//! let image = bmp::BmpImage::from_bytes(bmp_data).unwrap();
//! 
//! // Access CPU MSR
//! let apic_base = msr::read_msr(msr::IA32_APIC_BASE);
//! ```
//!
//! # Safety
//! This module contains unsafe code for:
//! - Direct hardware register access (MSR operations)
//! - Raw memory manipulation in image parsing
//! - Low-level logging infrastructure
//!
//! All unsafe operations are properly bounded and documented with
//! their safety requirements.

pub mod bmp;
pub mod initrd;
pub mod logger;
pub mod msr;

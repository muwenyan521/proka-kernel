//! Graphics module for the Proka kernel
//!
//! This module provides graphics rendering capabilities for the kernel, including
//! color management, pixel manipulation, and basic rendering operations.
//!
//! # Submodules
//! - [`color`]: Color representation and manipulation utilities
//! - [`core`]: Core graphics rendering functionality
//!
//! # Key Types
//! - [`Color`]: Represents a 32-bit ARGB color value
//! - [`Pixel`]: Represents a single pixel on the screen
//! - [`Renderer`]: Main rendering interface for drawing operations
//!
//! # Features
//! - **Color Management**: Support for ARGB color format with alpha blending
//! - **Pixel Operations**: Low-level pixel manipulation functions
//! - **Basic Shapes**: Drawing primitives (lines, rectangles, circles)
//! - **Text Rendering**: Basic text output using kernel fonts
//!
//! # Examples
//! ```rust
//! use crate::graphics::{Color, Renderer};
//!
//! // Create a renderer for a framebuffer
//! let mut renderer = Renderer::new(framebuffer_addr, width, height, pitch);
//! 
//! // Set drawing color
//! renderer.set_color(Color::rgb(255, 0, 0)); // Red
//! 
//! // Draw a rectangle
//! renderer.fill_rect(100, 100, 200, 150);
//! 
//! // Draw text
//! renderer.draw_text(50, 50, "Hello, Proka!");
//! ```
//!
//! # Safety
//! This module contains unsafe code for:
//! - Direct framebuffer memory access
//! - MMIO (Memory-Mapped I/O) operations for graphics hardware
//! - Raw pointer manipulation for pixel data
//!
//! All unsafe operations require proper synchronization and should only be
//! called when graphics hardware is in a known state.

pub mod color;
pub mod core;

pub use color::Color;
pub use core::{Pixel, Renderer};

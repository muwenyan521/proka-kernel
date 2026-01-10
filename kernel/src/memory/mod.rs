//! Memory management module for the Proka kernel
//!
//! This module provides comprehensive memory management facilities for the kernel,
//! including physical memory allocation, virtual memory paging, memory protection,
//! and kernel heap allocation.
//!
//! # Submodules
//! - [`allocator`]: Kernel heap allocator implementations
//! - [`frame_allocator`]: Physical frame (page) allocation
//! - [`paging`]: Virtual memory paging and page table management
//! - [`protection`]: Memory protection and access control
//!
//! # Memory Architecture
//! The memory management system follows a hierarchical structure:
//! 1. **Physical Layer**: Frame allocator manages physical memory pages
//! 2. **Virtual Layer**: Paging system maps virtual to physical addresses
//! 3. **Allocation Layer**: Heap allocator provides dynamic memory allocation
//! 4. **Protection Layer**: Memory protection enforces access permissions
//!
//! # Key Features
//! - **Physical Memory Management**: Buddy system or bitmap-based frame allocation
//! - **Virtual Memory**: 4-level paging (x86_64) with page table management
//! - **Heap Allocation**: Kernel heap with support for multiple allocators
//! - **Memory Protection**: Page-level permissions (read/write/execute)
//! - **Address Translation**: Virtual to physical address conversion
//!
//! # Examples
//! ```rust
//! use crate::memory::{frame_allocator, paging, allocator};
//!
//! // Initialize memory management
//! frame_allocator::init(memory_map);
//! paging::init();
//! allocator::init();
//! 
//! // Allocate a physical frame
//! let frame = frame_allocator::allocate_frame().unwrap();
//! 
//! // Map a virtual address to the physical frame
//! paging::map_page(0x1000, frame, paging::Flags::WRITABLE);
//! 
//! // Allocate kernel heap memory
//! let ptr = allocator::alloc(1024);
//! ```
//!
//! # Safety
//! This module contains extensive unsafe code for:
//! - Direct manipulation of page tables and memory mappings
//! - Physical memory management and frame allocation
//! - Low-level memory operations and pointer manipulation
//! - Hardware-specific memory management features
//!
//! All memory operations must ensure proper synchronization and
//! follow strict safety protocols to prevent memory corruption.

pub mod allocator;
pub mod frame_allocator;
pub mod paging;
pub mod protection;

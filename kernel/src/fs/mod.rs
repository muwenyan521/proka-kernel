//! File system module for the Proka kernel
//!
//! This module provides the file system infrastructure for the kernel, including
//! block I/O operations, virtual file system abstraction, and various file system
//! implementations.
//!
//! # Submodules
//! - [`bio`]: Block I/O operations for reading and writing data blocks
//! - [`kernfs`]: Kernel file system for internal kernel data structures
//! - [`memfs`]: In-memory file system for temporary storage
//! - [`vfs`]: Virtual File System abstraction layer
//!
//! # Architecture
//! The file system module follows a layered architecture:
//! 1. **VFS Layer**: Provides a unified interface to different file system types
//! 2. **File System Implementations**: Specific file systems (kernfs, memfs)
//! 3. **Block I/O Layer**: Low-level block device operations
//!
//! # Examples
//! ```rust
//! use crate::fs::vfs::VfsNode;
//! use crate::fs::memfs::MemFs;
//!
//! // Create an in-memory file system
//! let mut fs = MemFs::new();
//! // Create a file
//! fs.create_file("test.txt", b"Hello, world!").unwrap();
//! // Read the file
//! let content = fs.read_file("test.txt").unwrap();
//! assert_eq!(content, b"Hello, world!");
//! ```
//!
//! # Safety
//! This module contains unsafe code for:
//! - Direct memory access in block I/O operations
//! - Raw pointer manipulation in VFS layer
//! - Interrupt handling during disk operations
//!
//! All unsafe operations are properly documented and should only be called
//! from trusted kernel code.

pub mod bio;
pub mod kernfs;
pub mod memfs;
pub mod vfs;

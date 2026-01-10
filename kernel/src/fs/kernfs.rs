//! Kernel filesystem implementation
//!
//! This module provides `kernfs`, an in-memory filesystem for exposing
//! kernel data structures and devices through a filesystem interface.
//! It's similar to Linux's sysfs or procfs, allowing user-space programs
//! to interact with kernel objects via standard file operations.
//!
//! # Overview
//!
//! Kernfs provides a virtual filesystem that can be mounted to expose:
//! - Dynamic files with custom read/write callbacks
//! - Directory structures for organizing kernel objects
//! - Device nodes that map to actual hardware devices
//! - Hierarchical namespace for kernel data
//!
//! # Architecture
//!
//! The filesystem consists of:
//! 1. `KernFs` - The filesystem instance
//! 2. `KernInode` - Filesystem nodes (files, directories, devices)
//! 3. `KernNodeContent` - Content type of each node
//!
//! # Usage
//!
//! ```rust
//! use crate::fs::kernfs::{KernFs, KernInode};
//! use crate::fs::vfs::{FileSystem, VNodeType};
//! use alloc::sync::Arc;
//!
//! // Create a new kernfs instance
//! let kernfs = KernFs::new();
//!
//! // Mount it (returns root inode)
//! let root = kernfs.mount(None, None).unwrap();
//!
//! // Create a file with custom read callback
//! let read_cb = Box::new(|offset, buf| {
//!     // Custom read logic
//!     Ok(0)
//! });
//! let file = KernInode::new_file(Some(read_cb), None);
//!
//! // Add it to the filesystem
//! root.lookup(".").unwrap().as_any().downcast_ref::<KernInode>().unwrap()
//!     .add_child("myfile", file).unwrap();
//! ```
//!
//! # Features
//!
//! - **Dynamic files**: Files with custom read/write callbacks
//! - **Device nodes**: Map to character devices for I/O operations
//! - **Hierarchical structure**: Full directory support
//! - **Thread-safe**: Uses `RwLock` for concurrent access
//!
//! # Limitations
//!
//! - In-memory only (no persistence)
//! - No permission system (all files are 0o755)
//! - No hard links or symbolic links
//! - Limited metadata support

extern crate alloc;
use crate::drivers::Device;
use crate::fs::vfs::{FileSystem, Inode, Metadata, VNodeType, VfsError};
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::any::Any;
use spin::RwLock;

/// Callback type for reading from a kernfs file
///
/// This callback is invoked when a file is read. It receives:
/// - `offset`: The byte offset within the file
/// - `buf`: Buffer to fill with data
///
/// Returns the number of bytes read or an error.
pub type ReadCallback = Box<dyn Fn(u64, &mut [u8]) -> Result<usize, VfsError> + Send + Sync>;

/// Callback type for writing to a kernfs file
///
/// This callback is invoked when a file is written. It receives:
/// - `offset`: The byte offset within the file
/// - `buf`: Buffer containing data to write
///
/// Returns the number of bytes written or an error.
pub type WriteCallback = Box<dyn Fn(u64, &[u8]) -> Result<usize, VfsError> + Send + Sync>;

/// Content type of a kernfs node
///
/// This enum defines what type of content a `KernInode` contains.
pub enum KernNodeContent {
    /// Directory containing child nodes
    Dir(RwLock<BTreeMap<String, Arc<KernInode>>>),
    
    /// File with optional read/write callbacks
    File {
        /// Optional callback for reading from the file
        read: Option<ReadCallback>,
        /// Optional callback for writing to the file
        write: Option<WriteCallback>,
        /// Current file size in bytes
        size: u64,
    },
    
    /// Device node mapping to a hardware device
    Device {
        /// The underlying device
        device: Arc<Device>,
    },
}

/// Kernel filesystem inode
///
/// Represents a node in the kernfs filesystem. This can be a file,
/// directory, or device node. Each inode implements the `Inode` trait
/// to provide standard filesystem operations.
pub struct KernInode {
    /// Type of this node (file, directory, or device)
    node_type: VNodeType,
    /// Content of this node
    content: KernNodeContent,
}

impl KernInode {
    /// Creates a new directory node
    ///
    /// # Returns
    /// A new directory inode wrapped in `Arc`
    ///
    /// # Examples
    /// ```rust
    /// use crate::fs::kernfs::KernInode;
    /// use alloc::sync::Arc;
    ///
    /// let dir = KernInode::new_dir();
    /// ```
    pub fn new_dir() -> Arc<Self> {
        Arc::new(Self {
            node_type: VNodeType::Dir,
            content: KernNodeContent::Dir(RwLock::new(BTreeMap::new())),
        })
    }

    /// Creates a new file node with optional read/write callbacks
    ///
    /// # Arguments
    /// * `read` - Optional callback for reading from the file
    /// * `write` - Optional callback for writing to the file
    ///
    /// # Returns
    /// A new file inode wrapped in `Arc`
    ///
    /// # Examples
    /// ```rust
    /// use crate::fs::kernfs::{KernInode, ReadCallback};
    /// use crate::fs::vfs::VfsError;
    /// use alloc::sync::Arc;
    ///
    /// let read_cb: ReadCallback = Box::new(|offset, buf| {
    ///     // Simple read: fill buffer with zeros
    ///     for b in buf.iter_mut() {
    ///         *b = 0;
    ///     }
    ///     Ok(buf.len())
    /// });
    ///
    /// let file = KernInode::new_file(Some(read_cb), None);
    /// ```
    pub fn new_file(read: Option<ReadCallback>, write: Option<WriteCallback>) -> Arc<Self> {
        Arc::new(Self {
            node_type: VNodeType::File,
            content: KernNodeContent::File {
                read,
                write,
                size: 0,
            },
        })
    }

    /// Creates a new device node
    ///
    /// # Arguments
    /// * `device` - The device to map to this node
    ///
    /// # Returns
    /// A new device inode wrapped in `Arc`
    ///
    /// # Examples
    /// ```rust
    /// use crate::fs::kernfs::KernInode;
    /// use crate::drivers::Device;
    /// use alloc::sync::Arc;
    ///
    /// let device: Arc<Device> = /* get device */;
    /// let dev_node = KernInode::new_device(device);
    /// ```
    pub fn new_device(device: Arc<Device>) -> Arc<Self> {
        Arc::new(Self {
            node_type: VNodeType::Device,
            content: KernNodeContent::Device { device },
        })
    }

    /// Adds a child node to a directory
    ///
    /// This method can only be called on directory nodes. It adds
    /// a new child with the given name.
    ///
    /// # Arguments
    /// * `name` - Name of the child node
    /// * `child` - The child node to add
    ///
    /// # Returns
    /// * `Ok(())` - If the child was added successfully
    /// * `Err(VfsError::AlreadyExists)` - If a child with that name already exists
    /// * `Err(VfsError::NotADirectory)` - If this node is not a directory
    ///
    /// # Examples
    /// ```rust
    /// use crate::fs::kernfs::KernInode;
    /// use alloc::sync::Arc;
    ///
    /// let dir = KernInode::new_dir();
    /// let file = KernInode::new_file(None, None);
    ///
    /// dir.add_child("myfile", file).unwrap();
    /// ```
    pub fn add_child(&self, name: &str, child: Arc<KernInode>) -> Result<(), VfsError> {
        match &self.content {
            KernNodeContent::Dir(entries) => {
                let mut map = entries.write();
                if map.contains_key(name) {
                    return Err(VfsError::AlreadyExists);
                }
                map.insert(name.to_string(), child);
                Ok(())
            }
            _ => Err(VfsError::NotADirectory),
        }
    }
}

impl Inode for KernInode {
    /// Gets metadata for this inode
    ///
    /// Returns basic filesystem metadata. Note that kernfs has limited
    /// metadata support - all files have permissions 0o755 and timestamps
    /// are not implemented.
    fn metadata(&self) -> Result<Metadata, VfsError> {
        let size = match &self.content {
            KernNodeContent::File { size, .. } => *size,
            _ => 0,
        };
        Ok(Metadata {
            size,
            permissions: 0o755,
            uid: 0,
            gid: 0,
            ctime: 0,
            mtime: 0,
            blocks: 0,
            nlinks: 1,
        })
    }

    /// Sets metadata for this inode (not implemented)
    ///
    /// Kernfs does not support setting metadata.
    fn set_metadata(&self, _metadata: &Metadata) -> Result<(), VfsError> {
        Err(VfsError::NotImplemented)
    }

    /// Gets the node type (file, directory, or device)
    fn node_type(&self) -> VNodeType {
        self.node_type
    }

    /// Reads data from this inode at the specified offset
    ///
    /// For files, this invokes the read callback if one is set.
    /// For devices, this reads from the underlying character device.
    /// For directories, this returns `VfsError::NotAFile`.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, VfsError> {
        match &self.content {
            KernNodeContent::File { read, .. } => {
                if let Some(cb) = read {
                    cb(offset, buf)
                } else {
                    Err(VfsError::PermissionDenied)
                }
            }
            KernNodeContent::Device { device } => {
                if let Some(char_dev) = device.as_char_device() {
                    char_dev.read(buf).map_err(VfsError::DeviceError)
                } else {
                    Err(VfsError::NotImplemented)
                }
            }
            _ => Err(VfsError::NotAFile),
        }
    }

    /// Writes data to this inode at the specified offset
    ///
    /// For files, this invokes the write callback if one is set.
    /// For devices, this writes to the underlying character device.
    /// For directories, this returns `VfsError::NotAFile`.
    fn write_at(&self, offset: u64, buf: &[u8]) -> Result<usize, VfsError> {
        match &self.content {
            KernNodeContent::File { write, .. } => {
                if let Some(cb) = write {
                    cb(offset, buf)
                } else {
                    Err(VfsError::PermissionDenied)
                }
            }
            KernNodeContent::Device { device } => {
                if let Some(char_dev) = device.as_char_device() {
                    char_dev.write(buf).map_err(VfsError::DeviceError)
                } else {
                    Err(VfsError::NotImplemented)
                }
            }
            _ => Err(VfsError::NotAFile),
        }
    }

    /// Truncates this file to the specified size (not implemented)
    fn truncate(&self, _size: u64) -> Result<(), VfsError> {
        Err(VfsError::NotImplemented)
    }

    /// Synchronizes this inode (no-op for kernfs)
    fn sync(&self) -> Result<(), VfsError> {
        Ok(())
    }

    /// Looks up a child node by name
    ///
    /// This can only be called on directory nodes.
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError> {
        match &self.content {
            KernNodeContent::Dir(entries) => entries
                .read()
                .get(name)
                .cloned()
                .map(|n| n as Arc<dyn Inode>)
                .ok_or(VfsError::NotFound),
            _ => Err(VfsError::NotADirectory),
        }
    }

    /// Creates a new child node
    ///
    /// This can only be called on directory nodes.
    fn create(&self, name: &str, typ: VNodeType) -> Result<Arc<dyn Inode>, VfsError> {
        match &self.content {
            KernNodeContent::Dir(entries) => {
                let mut map = entries.write();
                if map.contains_key(name) {
                    return Err(VfsError::AlreadyExists);
                }

                let new_inode = match typ {
                    VNodeType::Dir => KernInode::new_dir(),
                    VNodeType::File => KernInode::new_file(None, None),
                    _ => return Err(VfsError::NotImplemented),
                };

                map.insert(name.to_string(), new_inode.clone());
                Ok(new_inode)
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    /// Creates a new device node
    ///
    /// This can only be called on directory nodes.
    fn create_device(
        &self,
        name: &str,
        device: Arc<Device>,
    ) -> Result<Arc<dyn Inode>, VfsError> {
        match &self.content {
            KernNodeContent::Dir(entries) => {
                let mut map = entries.write();
                if map.contains_key(name) {
                    return Err(VfsError::AlreadyExists);
                }
                
                let new_inode = KernInode::new_device(device);
                map.insert(name.to_string(), new_inode.clone());
                Ok(new_inode)
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    /// Unlinks (removes) a child node
    ///
    /// This can only be called on directory nodes.
    fn unlink(&self, name: &str) -> Result<(), VfsError> {
        match &self.content {
            KernNodeContent::Dir(entries) => {
                let mut map = entries.write();
                if map.remove(name).is_some() {
                    Ok(())
                } else {
                    Err(VfsError::NotFound)
                }
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    /// Lists all child nodes in a directory
    ///
    /// This can only be called on directory nodes.
    fn list(&self) -> Result<Vec<String>, VfsError> {
        match &self.content {
            KernNodeContent::Dir(entries) => Ok(entries.read().keys().cloned().collect()),
            _ => Err(VfsError::NotADirectory),
        }
    }

    /// Returns this inode as `&dyn Any` for downcasting
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Kernel filesystem instance
///
/// This struct represents a kernfs filesystem instance. It contains
/// a root directory and implements the `FileSystem` trait.
pub struct KernFs {
    root: Arc<KernInode>,
}

impl KernFs {
    /// Creates a new kernfs instance
    ///
    /// # Returns
    /// A new `KernFs` instance with an empty root directory
    ///
    /// # Examples
    /// ```rust
    /// use crate::fs::kernfs::KernFs;
    ///
    /// let kernfs = KernFs::new();
    /// ```
    pub fn new() -> Self {
        let root = KernInode::new_dir();

        Self { root }
    }

    /// Gets the root inode of this filesystem
    ///
    /// # Returns
    /// The root directory inode
    ///
    /// # Examples
    /// ```rust
    /// use crate::fs::kernfs::KernFs;
    /// use alloc::sync::Arc;
    ///
    /// let kernfs = KernFs::new();
    /// let root = kernfs.root();
    /// ```
    pub fn root(&self) -> Arc<KernInode> {
        self.root.clone()
    }
}

impl FileSystem for KernFs {
    /// Mounts this filesystem
    ///
    /// For kernfs, mounting always succeeds and returns the root inode.
    /// The `device` and `args` parameters are ignored.
    ///
    /// # Arguments
    /// * `device` - Optional block device (ignored for kernfs)
    /// * `args` - Optional mount arguments (ignored for kernfs)
    ///
    /// # Returns
    /// The root inode of the filesystem
    fn mount(
        &self,
        _device: Option<Arc<Device>>,
        _args: Option<&[&str]>,
    ) -> Result<Arc<dyn Inode>, VfsError> {
        Ok(self.root.clone())
    }

    /// Gets the filesystem type
    ///
    /// # Returns
    /// The string "kernfs"
    fn fs_type(&self) -> &'static str {
        "kernfs"
    }
}

//! Device management system for the Proka kernel.
//!
//! This module provides a unified device abstraction layer that handles:
//! - Device registration and unregistration
//! - Major/minor number allocation
//! - Device type classification (block vs character devices)
//! - Reference counting for device usage
//! - Device operations through trait interfaces
//!
//! # Architecture
//!
//! The device system is built around several key components:
//! 1. [`DeviceType`] - Enumeration of device types (block, character)
//! 2. [`DeviceError`] - Error types for device operations
//! 3. [`SharedDeviceOps`] - Common operations for all devices
//! 4. [`BlockDevice`] - Trait for block-oriented devices
//! 5. [`CharDevice`] - Trait for character-oriented devices
//! 6. [`Device`] - Wrapper struct managing device state
//! 7. [`DeviceManager`] - Central registry for all devices
//!
//! # Examples
//!
//! ```rust
//! use crate::drivers::device::{DeviceManager, DeviceType, DeviceError};
//! use crate::drivers::device::DEVICE_MANAGER;
//!
//! // Register a new device
//! let mut manager = DEVICE_MANAGER.write();
//! // ... create device implementation
//! // manager.register_device(device)?;
//! ```
//!
//! # Safety
//!
//! - Device operations must be thread-safe as devices can be accessed concurrently
//! - Reference counting ensures devices aren't unregistered while in use
//! - Atomic operations are used for open counts and registration state

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use lazy_static::lazy_static;
use spin::RwLock;

lazy_static! {
    /// Global device manager instance.
    ///
    /// This is a thread-safe, lazily initialized device registry that manages
    /// all devices in the system. Use `DEVICE_MANAGER.write()` to get exclusive
    /// access for registration/unregistration, and `DEVICE_MANAGER.read()` for
    /// concurrent read access.
    pub static ref DEVICE_MANAGER: RwLock<DeviceManager> = RwLock::new(DeviceManager::new());
}

/// Type of device in the system.
///
/// Devices are classified based on their access patterns and functionality:
/// - Block devices: Random access, fixed-size blocks (e.g., disks)
/// - Character devices: Stream-oriented, byte-by-byte access (e.g., serial ports)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// Block-oriented storage device.
    ///
    /// These devices support random access to fixed-size blocks of data.
    /// Typical examples include hard drives, SSDs, and RAM disks.
    Block,
    
    /// Character-oriented I/O device.
    ///
    /// These devices provide sequential, byte-stream access.
    /// Typical examples include serial ports, keyboards, and displays.
    Char,
}

/// Error types for device operations.
///
/// This enum provides detailed error information for device-related failures,
/// allowing callers to handle different failure modes appropriately.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DeviceError {
    /// Invalid parameter passed to device operation.
    InvalidParam,
    /// Operation not supported by this device.
    NotSupported,
    /// General I/O error occurred.
    IoError,
    /// Insufficient permissions for operation.
    PermissionsDenied,
    /// Requested device does not exist.
    NoSuchDevice,
    /// Operation would block but device is in non-blocking mode.
    WouldBlock,
    /// Device is busy with another operation.
    Busy,
    /// Insufficient memory for operation.
    OutOfMemory,
    /// Device is not open.
    DeviceClosed,
    /// Provided buffer is too small for operation.
    BufferTooSmall,
    /// Device is already open.
    AlreadyOpen,
    /// Device is not open.
    NotOpen,
    /// Address is outside valid range for device.
    AddressOutOfRange,
    /// Device with this name is already registered.
    DeviceAlreadyRegistered,
    /// Major/minor number combination is already in use.
    DeviceNumberConflict,
    /// Device is not registered with the device manager.
    DeviceNotRegistered,
    /// Device cannot be unregistered because it's still in use.
    DeviceStillInUse,
}

/// Information collected during device scanning.
///
/// This structure contains metadata about discovered hardware devices,
/// used for device matching and driver compatibility checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanInfo {
    /// Unique identifier for the device.
    pub device_id: String,
    /// Communication protocol type (e.g., "USB", "PCI", "I2C").
    pub protocol_type: String,
    /// Vendor ID (if available).
    pub vendor_id: Option<u16>,
    /// Product ID (if available).
    pub product_id: Option<u16>,
    /// Additional device-specific metadata.
    pub additional_data: Option<BTreeMap<String, String>>,
}

/// Common operations shared by all device types.
///
/// This trait defines the minimal interface that every device must implement,
/// regardless of whether it's a block or character device.
pub trait SharedDeviceOps: Send + Sync {
    /// Returns the name of the device.
    fn name(&self) -> &str;
    
    /// Returns the type of the device (block or character).
    fn device_type(&self) -> DeviceType;

    /// Opens the device for operations.
    ///
    /// # Returns
    /// - `Ok(())` if the device was successfully opened
    /// - `Err(DeviceError)` if opening failed
    fn open(&self) -> Result<(), DeviceError>;
    
    /// Closes the device.
    ///
    /// # Returns
    /// - `Ok(())` if the device was successfully closed
    /// - `Err(DeviceError)` if closing failed
    fn close(&self) -> Result<(), DeviceError>;
    
    /// Performs a device-specific control operation.
    ///
    /// # Arguments
    /// * `cmd` - Command code
    /// * `arg` - Command argument
    ///
    /// # Returns
    /// - `Ok(u64)` with command result on success
    /// - `Err(DeviceError)` if the operation failed
    fn ioctl(&self, cmd: u64, arg: u64) -> Result<u64, DeviceError>;

    /// Synchronizes device state (e.g., flushes caches).
    ///
    /// Default implementation returns `DeviceError::NotSupported`.
    ///
    /// # Returns
    /// - `Ok(())` if synchronization succeeded
    /// - `Err(DeviceError)` if synchronization failed or is not supported
    fn sync(&self) -> Result<(), DeviceError> {
        Err(DeviceError::NotSupported)
    }
    
    /// Checks if this device driver is compatible with the given scan information.
    ///
    /// Default implementation returns `false`.
    ///
    /// # Arguments
    /// * `scan_info` - Device scanning information
    ///
    /// # Returns
    /// `true` if the driver is compatible, `false` otherwise
    fn is_compatible(&self, _scan_info: &ScanInfo) -> bool {
        false
    }
}

/// Operations for block-oriented storage devices.
///
/// Block devices provide random access to fixed-size blocks of data.
/// Typical implementations include disk drivers and RAM disk drivers.
pub trait BlockDevice: SharedDeviceOps {
    /// Returns the size of each block in bytes.
    fn block_size(&self) -> usize;
    
    /// Returns the total number of blocks available on the device.
    fn num_blocks(&self) -> usize;

    /// Reads one or more blocks from the device.
    ///
    /// # Arguments
    /// * `block_idx` - Starting block index (0-based)
    /// * `num_blocks` - Number of blocks to read
    /// * `buf` - Buffer to store read data (must be at least `num_blocks * block_size()` bytes)
    ///
    /// # Returns
    /// - `Ok(usize)` with number of blocks actually read
    /// - `Err(DeviceError)` if the read operation failed
    fn read_blocks(
        &self,
        block_idx: usize,
        num_blocks: usize,
        buf: &mut [u8],
    ) -> Result<usize, DeviceError>;

    /// Writes one or more blocks to the device.
    ///
    /// # Arguments
    /// * `block_idx` - Starting block index (0-based)
    /// * `num_blocks` - Number of blocks to write
    /// * `buf` - Buffer containing data to write (must be at least `num_blocks * block_size()` bytes)
    ///
    /// # Returns
    /// - `Ok(usize)` with number of blocks actually written
    /// - `Err(DeviceError)` if the write operation failed
    fn write_blocks(
        &self,
        block_idx: usize,
        num_blocks: usize,
        buf: &[u8],
    ) -> Result<usize, DeviceError>;

    /// Erases one or more blocks on the device.
    ///
    /// This operation is typically supported by flash memory devices.
    /// Default implementation returns `DeviceError::NotSupported`.
    ///
    /// # Arguments
    /// * `start_block` - Starting block index (0-based)
    /// * `num_blocks` - Number of blocks to erase
    ///
    /// # Returns
    /// - `Ok(usize)` with number of blocks actually erased
    /// - `Err(DeviceError)` if the erase operation failed or is not supported
    fn erase_blocks(&self, start_block: usize, num_blocks: usize) -> Result<usize, DeviceError> {
        let _ = (start_block, num_blocks);
        Err(DeviceError::NotSupported)
    }
}

/// Operations for character-oriented I/O devices.
///
/// Character devices provide sequential, byte-stream access.
/// Typical implementations include serial ports, keyboards, and displays.
pub trait CharDevice: SharedDeviceOps {
    /// Reads bytes from the device into the buffer.
    ///
    /// # Arguments
    /// * `buf` - Buffer to store read data
    ///
    /// # Returns
    /// - `Ok(usize)` with number of bytes actually read
    /// - `Err(DeviceError)` if the read operation failed
    fn read(&self, buf: &mut [u8]) -> Result<usize, DeviceError>;
    
    /// Writes bytes from the buffer to the device.
    ///
    /// # Arguments
    /// * `buf` - Buffer containing data to write
    ///
    /// # Returns
    /// - `Ok(usize)` with number of bytes actually written
    /// - `Err(DeviceError)` if the write operation failed
    fn write(&self, buf: &[u8]) -> Result<usize, DeviceError>;

    /// Peeks at available data without consuming it.
    ///
    /// Default implementation returns `DeviceError::NotSupported`.
    ///
    /// # Arguments
    /// * `buf` - Buffer to store peeked data
    ///
    /// # Returns
    /// - `Ok(usize)` with number of bytes available for reading
    /// - `Err(DeviceError)` if peeking is not supported
    fn peek(&self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let _ = buf;
        Err(DeviceError::NotSupported)
    }

    /// Checks if data is available for reading.
    ///
    /// Default implementation returns `false`.
    ///
    /// # Returns
    /// `true` if data is available, `false` otherwise
    fn has_data(&self) -> bool {
        false
    }

    /// Checks if space is available for writing.
    ///
    /// Default implementation returns `false`.
    ///
    /// # Returns
    /// `true` if space is available, `false` otherwise
    fn has_space(&self) -> bool {
        false
    }

    /// Sets the device's blocking/non-blocking mode.
    ///
    /// Default implementation returns `DeviceError::NotSupported`.
    ///
    /// # Arguments
    /// * `nonblocking` - `true` for non-blocking mode, `false` for blocking mode
    ///
    /// # Returns
    /// - `Ok(())` if mode was successfully set
    /// - `Err(DeviceError)` if mode setting failed or is not supported
    fn set_nonblocking(&self, nonblocking: bool) -> Result<(), DeviceError> {
        let _ = nonblocking;
        Err(DeviceError::NotSupported)
    }
}

/// Internal representation of a device.
///
/// This enum wraps the actual device implementation, allowing the [`Device`]
/// struct to handle both block and character devices uniformly.
#[derive(Clone)]
pub enum DeviceInner {
    /// Character device implementation.
    Char(Arc<dyn CharDevice>),
    /// Block device implementation.
    Block(Arc<dyn BlockDevice>),
}

/// A registered device in the system.
///
/// This struct wraps a device implementation with metadata and state management,
/// including reference counting for open operations and registration status.
pub struct Device {
    /// Human-readable device name.
    pub name: String,
    /// Major number identifying the device driver class.
    pub major: u16,
    /// Minor number identifying a specific device instance.
    pub minor: u16,
    /// The actual device implementation.
    pub inner: DeviceInner,
    /// Number of times this device has been opened.
    open_count: AtomicUsize,
    /// Whether the device is currently registered with the device manager.
    is_registered: AtomicBool,
}

impl Device {
    /// Creates a new device with specified major and minor numbers.
    ///
    /// # Arguments
    /// * `name` - Human-readable device name
    /// * `major` - Major number for device classification
    /// * `minor` - Minor number for device instance
    /// * `inner` - The actual device implementation
    ///
    /// # Returns
    /// A new `Device` instance with the given parameters.
    pub fn new(name: String, major: u16, minor: u16, inner: DeviceInner) -> Self {
        Self {
            name,
            major,
            minor,
            inner,
            open_count: AtomicUsize::new(0),
            is_registered: AtomicBool::new(false),
        }
    }

    /// Creates a new device with automatic major/minor number assignment.
    ///
    /// The device manager will assign appropriate major/minor numbers
    /// when this device is registered.
    ///
    /// # Arguments
    /// * `name` - Human-readable device name
    /// * `inner` - The actual device implementation
    ///
    /// # Returns
    /// A new `Device` instance with unassigned major/minor numbers.
    pub fn new_auto_assign(name: String, inner: DeviceInner) -> Self {
        Self {
            name,
            major: 0,
            minor: 0,
            inner,
            open_count: AtomicUsize::new(0),
            is_registered: AtomicBool::new(false),
        }
    }

    /// Returns a reference to the shared device operations.
    #[inline]
    fn shared_ops(&self) -> &dyn SharedDeviceOps {
        match &self.inner {
            DeviceInner::Block(ops) => ops.as_ref(),
            DeviceInner::Char(ops) => ops.as_ref(),
        }
    }

    /// Returns the type of this device.
    pub fn device_type(&self) -> DeviceType {
        self.shared_ops().device_type()
    }

    /// Opens the device for I/O operations.
    ///
    /// This method implements reference counting, so multiple calls to `open()`
    /// will only call the underlying device's `open()` method once.
    ///
    /// # Returns
    /// - `Ok(())` if the device was successfully opened
    /// - `Err(DeviceError::DeviceNotRegistered)` if the device is not registered
    /// - `Err(DeviceError)` if the underlying device's `open()` method fails
    pub fn open(&self) -> Result<(), DeviceError> {
        if !self.is_registered.load(Ordering::SeqCst) {
            return Err(DeviceError::DeviceNotRegistered);
        }

        let current_count = self.open_count.fetch_add(1, Ordering::SeqCst);
        if current_count == 0 {
            self.shared_ops().open()?;
        }
        Ok(())
    }

    /// Closes the device.
    ///
    /// This method implements reference counting, so the underlying device's
    /// `close()` method is only called when the last open reference is closed.
    ///
    /// # Returns
    /// - `Ok(())` if the device was successfully closed
    /// - `Err(DeviceError::DeviceNotRegistered)` if the device is not registered
    /// - `Err(DeviceError::NotOpen)` if the device is not open
    /// - `Err(DeviceError)` if the underlying device's `close()` method fails
    pub fn close(&self) -> Result<(), DeviceError> {
        if !self.is_registered.load(Ordering::SeqCst) {
            return Err(DeviceError::DeviceNotRegistered);
        }

        let current_count = self.open_count.fetch_sub(1, Ordering::SeqCst);
        if current_count == 1 {
            self.shared_ops().close()?;
        } else if current_count == 0 {
            return Err(DeviceError::NotOpen);
        }
        Ok(())
    }

    /// Performs a device-specific control operation.
    ///
    /// # Arguments
    /// * `cmd` - Command code
    /// * `arg` - Command argument
    ///
    /// # Returns
    /// - `Ok(u64)` with command result on success
    /// - `Err(DeviceError::DeviceNotRegistered)` if the device is not registered
    /// - `Err(DeviceError::DeviceClosed)` if the device is not open
    /// - `Err(DeviceError)` if the underlying device's `ioctl()` method fails
    pub fn ioctl(&self, cmd: u64, arg: u64) -> Result<u64, DeviceError> {
        if !self.is_registered.load(Ordering::SeqCst) {
            return Err(DeviceError::DeviceNotRegistered);
        }
        if self.open_count.load(Ordering::SeqCst) == 0 {
            return Err(DeviceError::DeviceClosed);
        }
        self.shared_ops().ioctl(cmd, arg)
    }

    /// Checks if the device is currently open.
    ///
    /// # Returns
    /// `true` if the device has at least one open reference, `false` otherwise.
    pub fn is_open(&self) -> bool {
        self.open_count.load(Ordering::Relaxed) > 0
    }

    /// Returns a reference to the block device implementation, if this is a block device.
    ///
    /// # Returns
    /// `Some(&Arc<dyn BlockDevice>)` if this is a block device, `None` otherwise.
    pub fn as_block_device(&self) -> Option<&Arc<dyn BlockDevice>> {
        if let DeviceInner::Block(ref ops) = self.inner {
            Some(ops)
        } else {
            None
        }
    }

    /// Returns a reference to the character device implementation, if this is a character device.
    ///
    /// # Returns
    /// `Some(&Arc<dyn CharDevice>)` if this is a character device, `None` otherwise.
    pub fn as_char_device(&self) -> Option<&Arc<dyn CharDevice>> {
        if let DeviceInner::Char(ref ops) = self.inner {
            Some(ops)
        } else {
            None
        }
    }

    /// Marks the device as registered with the device manager.
    ///
    /// This is called internally by [`DeviceManager::register_device`].
    fn mark_registered(&self) {
        self.is_registered.store(true, Ordering::SeqCst);
    }

    /// Marks the device as unregistered from the device manager.
    ///
    /// This is called internally by [`DeviceManager::unregister_device`].
    fn mark_unregistered(&self) {
        self.is_registered.store(false, Ordering::SeqCst);
    }
}

impl core::fmt::Debug for Device {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Device {{ name: {}, major: {}, minor: {}, open_count: {}, is_registered: {} }}",
            self.name,
            self.major,
            self.minor,
            self.open_count.load(Ordering::SeqCst),
            self.is_registered.load(Ordering::SeqCst)
        )
    }
}

impl Clone for Device {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            major: self.major,
            minor: self.minor,
            open_count: AtomicUsize::new(self.open_count.load(Ordering::SeqCst)),
            is_registered: AtomicBool::new(self.is_registered.load(Ordering::SeqCst)),
            inner: self.inner.clone(),
        }
    }
}

/// Central registry for managing all devices in the system.
///
/// The `DeviceManager` maintains a collection of all registered devices,
/// handles major/minor number allocation, and provides device lookup
/// capabilities. It ensures device numbers are unique and manages
/// device lifecycle.
///
/// # Thread Safety
///
/// The device manager is protected by a [`RwLock`], allowing multiple
/// concurrent readers but exclusive access for writers during device
/// registration/unregistration.
///
/// # Major/Minor Number Management
///
/// - Major numbers: Predefined based on device type (1 for character, 2 for block)
/// - Minor numbers: Dynamically allocated, with reuse of freed numbers
/// - Free minor numbers are tracked for efficient reuse
pub struct DeviceManager {
    /// All registered devices in the system.
    devices: Vec<Arc<Device>>,
    /// Next available minor number for each major number.
    next_minor_counters: BTreeMap<u16, u16>,
    /// Pool of freed minor numbers available for reuse, organized by major number.
    free_minors: BTreeMap<u16, Vec<u16>>,
}

impl DeviceManager {
    /// Creates a new, empty device manager.
    ///
    /// # Returns
    /// A new `DeviceManager` instance with no registered devices.
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            next_minor_counters: BTreeMap::new(),
            free_minors: BTreeMap::new(),
        }
    }

    /// Registers a new device with the device manager.
    ///
    /// If the device has major/minor numbers set to 0, they will be automatically
    /// assigned based on the device type. Otherwise, the provided numbers are used
    /// after verifying they are not already in use.
    ///
    /// # Arguments
    /// * `device` - The device to register
    ///
    /// # Returns
    /// - `Ok(Arc<Device>)` with a reference-counted handle to the registered device
    /// - `Err(DeviceError::DeviceAlreadyRegistered)` if a device with the same name exists
    /// - `Err(DeviceError::DeviceNumberConflict)` if the major/minor combination is already in use
    /// - `Err(DeviceError::OutOfMemory)` if no minor numbers are available
    ///
    /// # Examples
    /// ```rust
    /// use crate::drivers::device::{DeviceManager, Device, DeviceInner};
    /// use crate::drivers::device::DEVICE_MANAGER;
    ///
    /// let mut manager = DEVICE_MANAGER.write();
    /// // Create a device with auto-assigned numbers
    /// let device = Device::new_auto_assign("mydevice".to_string(), DeviceInner::Char(/* ... */));
    /// let registered = manager.register_device(device).expect("Failed to register device");
    /// ```
    pub fn register_device(&mut self, mut device: Device) -> Result<Arc<Device>, DeviceError> {
        if self.devices.iter().any(|d| d.name == device.name) {
            return Err(DeviceError::DeviceAlreadyRegistered);
        }

        if device.major == 0 && device.minor == 0 {
            let (major, minor) = self.alloc_device_number(device.device_type())?;
            device.major = major;
            device.minor = minor;
        } else {
            if self
                .devices
                .iter()
                .any(|d| d.major == device.major && d.minor == device.minor)
            {
                return Err(DeviceError::DeviceNumberConflict);
            }

            self.update_minor_counter(device.major, device.minor);
        }

        device.mark_registered();
        let device_arc = Arc::new(device);
        self.devices.push(device_arc.clone());
        Ok(device_arc)
    }

    /// Allocates a unique major/minor number pair for a new device.
    ///
    /// This method first checks the pool of freed minor numbers for reuse.
    /// If no free numbers are available, it searches for the next available
    /// minor number starting from the current counter.
    ///
    /// # Arguments
    /// * `device_type` - Type of device (character or block)
    ///
    /// # Returns
    /// - `Ok((major, minor))` with allocated device numbers
    /// - `Err(DeviceError::OutOfMemory)` if no minor numbers are available
    fn alloc_device_number(&mut self, device_type: DeviceType) -> Result<(u16, u16), DeviceError> {
        let major = match device_type {
            DeviceType::Char => 1,
            DeviceType::Block => 2,
        };

        if let Some(minor) = self.free_minors.get_mut(&major).and_then(|v| v.pop()) {
            return Ok((major, minor));
        }

        let next_minor = self.next_minor_counters.entry(major).or_insert(0);
        let mut current_minor = *next_minor;

        for _ in 0..u16::MAX as usize {
            let is_used = self
                .devices
                .iter()
                .any(|d| d.major == major && d.minor == current_minor);

            if !is_used {
                *next_minor = current_minor.checked_add(1).unwrap_or(0);
                return Ok((major, current_minor));
            }
            current_minor = current_minor.checked_add(1).unwrap_or(0);
        }

        Err(DeviceError::OutOfMemory)
    }

    /// Checks if a specific major/minor number combination is already in use.
    ///
    /// This method is useful for drivers that want to check if a particular
    /// device number is available before attempting to register a device.
    ///
    /// # Arguments
    /// * `major` - Major number to check
    /// * `minor` - Minor number to check
    ///
    /// # Returns
    /// `true` if the major/minor combination is already in use by a registered device,
    /// `false` otherwise.
    pub fn is_minor_used(&self, major: u16, minor: u16) -> bool {
        self.devices
            .iter()
            .any(|d| d.major == major && d.minor == minor)
    }

    /// Updates the next available minor number counter for a major number.
    ///
    /// This private method ensures that when a device is registered with a specific
    /// minor number, the counter for that major number is updated to avoid
    /// future conflicts.
    ///
    /// # Arguments
    /// * `major` - Major number
    /// * `minor` - Minor number that was just allocated
    fn update_minor_counter(&mut self, major: u16, minor: u16) {
        let counter = self.next_minor_counters.entry(major).or_insert(0);
        if minor >= *counter {
            *counter = minor + 1;
        }
    }

    /// Unregisters a device from the device manager.
    ///
    /// This removes the device from the system registry and makes its
    /// major/minor numbers available for reuse. The device cannot be
    /// unregistered if it is currently open (has active references).
    ///
    /// # Arguments
    /// * `name` - Name of the device to unregister
    ///
    /// # Returns
    /// - `Ok(())` if the device was successfully unregistered
    /// - `Err(DeviceError::NoSuchDevice)` if no device with the given name exists
    /// - `Err(DeviceError::DeviceStillInUse)` if the device is currently open
    ///
    /// # Examples
    /// ```rust
    /// use crate::drivers::device::DEVICE_MANAGER;
    ///
    /// let mut manager = DEVICE_MANAGER.write();
    /// match manager.unregister_device("mydevice") {
    ///     Ok(()) => println!("Device unregistered successfully"),
    ///     Err(e) => println!("Failed to unregister device: {:?}", e),
    /// }
    /// ```
    pub fn unregister_device(&mut self, name: &str) -> Result<(), DeviceError> {
        let position = self.devices.iter().position(|d| d.name == name);

        if let Some(index) = position {
            if self.devices[index].is_open() {
                return Err(DeviceError::DeviceStillInUse);
            }
            
            let device_arc = self.devices.remove(index);
            device_arc.mark_unregistered();
            self.reclaim_device_number(device_arc.major, device_arc.minor);
            Ok(())
        } else {
            Err(DeviceError::NoSuchDevice)
        }
    }

    /// Reclaims a major/minor number pair for future reuse.
    ///
    /// This private method adds a freed device number to the pool of available
    /// numbers, allowing efficient reuse and preventing number exhaustion.
    ///
    /// # Arguments
    /// * `major` - Major number to reclaim
    /// * `minor` - Minor number to reclaim
    fn reclaim_device_number(&mut self, major: u16, minor: u16) {
        self.free_minors
            .entry(major)
            .or_insert_with(Vec::new)
            .push(minor);
    }

    /// Retrieves a device by its name.
    ///
    /// # Arguments
    /// * `name` - Name of the device to retrieve
    ///
    /// # Returns
    /// - `Some(Arc<Device>)` if a device with the given name exists
    /// - `None` if no such device exists
    ///
    /// # Examples
    /// ```rust
    /// use crate::drivers::device::DEVICE_MANAGER;
    ///
    /// let manager = DEVICE_MANAGER.read();
    /// if let Some(device) = manager.get_device("serial0") {
    ///     println!("Found device: {}", device.name);
    /// }
    /// ```
    pub fn get_device(&self, name: &str) -> Option<Arc<Device>> {
        self.devices.iter().find(|d| d.name == name).cloned()
    }

    /// Retrieves a device by its major and minor numbers.
    ///
    /// This is useful for drivers that need to look up devices by their
    /// device numbers rather than by name.
    ///
    /// # Arguments
    /// * `major` - Major number of the device
    /// * `minor` - Minor number of the device
    ///
    /// # Returns
    /// - `Some(Arc<Device>)` if a device with the given major/minor exists
    /// - `None` if no such device exists
    ///
    /// # Examples
    /// ```rust
    /// use crate::drivers::device::DEVICE_MANAGER;
    ///
    /// let manager = DEVICE_MANAGER.read();
    /// // Look up the first serial device (major 1, minor 0)
    /// if let Some(device) = manager.get_device_by_major_minor(1, 0) {
    ///     println!("Found device: {}", device.name);
    /// }
    /// ```
    pub fn get_device_by_major_minor(&self, major: u16, minor: u16) -> Option<Arc<Device>> {
        self.devices
            .iter()
            .find(|d| d.major == major && d.minor == minor)
            .cloned()
    }

    /// Retrieves all devices of a specific type.
    ///
    /// # Arguments
    /// * `device_type` - Type of devices to retrieve (`DeviceType::Char` or `DeviceType::Block`)
    ///
    /// # Returns
    /// A vector containing all registered devices of the specified type.
    ///
    /// # Examples
    /// ```rust
    /// use crate::drivers::device::{DEVICE_MANAGER, DeviceType};
    ///
    /// let manager = DEVICE_MANAGER.read();
    /// let char_devices = manager.get_devices_by_type(DeviceType::Char);
    /// println!("Found {} character devices", char_devices.len());
    /// ```
    pub fn get_devices_by_type(&self, device_type: DeviceType) -> Vec<Arc<Device>> {
        self.devices
            .iter()
            .filter(|d| d.device_type() == device_type)
            .cloned()
            .collect()
    }

    /// Lists all registered devices in the system.
    ///
    /// # Returns
    /// A vector containing all registered devices, in the order they were registered.
    ///
    /// # Examples
    /// ```rust
    /// use crate::drivers::device::DEVICE_MANAGER;
    ///
    /// let manager = DEVICE_MANAGER.read();
    /// let all_devices = manager.list_devices();
    /// for device in all_devices {
    ///     println!("Device: {} (major: {}, minor: {})", 
    ///              device.name, device.major, device.minor);
    /// }
    /// ```
    pub fn list_devices(&self) -> Vec<Arc<Device>> {
        self.devices.clone()
    }
}

/// Initializes the default set of devices for the system.
///
/// This function is called during kernel initialization to register
/// essential hardware devices that are always present. Currently,
/// it registers:
/// 1. The first serial port (COM1 at 0x3f8)
/// 2. The keyboard device
///
/// # Panics
/// This function panics if any essential device fails to register,
/// as the system cannot function properly without these devices.
///
/// # Examples
/// ```rust
/// use crate::drivers::device::init_devices;
///
/// // Called during kernel boot
/// init_devices();
/// ```
pub fn init_devices() {
    let mut manager = DEVICE_MANAGER.write();
    manager
        .register_device(super::char::serial::SerialDevice::create_device(
            1, 0, 0x3f8,
        ))
        .expect("Failed to register serial device");

    manager
        .register_device(super::input::keyboard::Keyboard::create_device())
        .expect("Failed to register keyboard device");
}

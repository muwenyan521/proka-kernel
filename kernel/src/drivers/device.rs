extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use lazy_static::lazy_static;
use log::debug;
use spin::Mutex;

use crate::serial_println;

lazy_static! {
    pub static ref DEVICE_MANAGER: Mutex<DeviceManager> = Mutex::new(DeviceManager::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Block,
    Char,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DeviceError {
    InvalidParam,
    NotSupported,
    IoError,
    PermissionsDenied,
    NoSuchDevice,
    WouldBlock,
    Busy,
    OutOfMemory,
    DeviceClosed,
    BufferTooSmall,
    AlreadyOpen,
    NotOpen,
    AddressOutOfRange,
    DeviceAlreadyRegistered,
    DeviceNumberConflict,
    DeviceNotRegistered, // 新增：设备未注册错误
    DeviceStillInUse,    // 新增：设备仍在使用中
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanInfo {
    pub device_id: String,                                 // 设备唯一标识
    pub protocol_type: String,                             // 通信协议类型（如USB/PCI/I2C）
    pub vendor_id: Option<u16>,                            // 供应商ID
    pub product_id: Option<u16>,                           // 产品ID
    pub additional_data: Option<BTreeMap<String, String>>, // 附加数据
}

pub trait SharedDeviceOps: Send + Sync {
    fn name(&self) -> &str;
    fn device_type(&self) -> DeviceType;

    fn open(&self) -> Result<(), DeviceError>;
    fn close(&self) -> Result<(), DeviceError>;
    fn ioctl(&self, cmd: u64, arg: u64) -> Result<u64, DeviceError>;

    fn sync(&self) -> Result<(), DeviceError> {
        Err(DeviceError::NotSupported)
    }
    fn is_compatible(&self, _scan_info: &ScanInfo) -> bool {
        false
    }
}

pub trait BlockDeviceOps: SharedDeviceOps {
    fn block_size(&self) -> usize;
    fn num_blocks(&self) -> usize;

    fn read_blocks(
        &self,
        block_idx: usize,
        num_blocks: usize,
        buf: &mut [u8],
    ) -> Result<usize, DeviceError>;

    fn write_blocks(
        &self,
        block_idx: usize,
        num_blocks: usize,
        buf: &[u8],
    ) -> Result<usize, DeviceError>;

    // 新增擦除块操作
    fn erase_blocks(&self, start_block: usize, num_blocks: usize) -> Result<usize, DeviceError> {
        let _ = (start_block, num_blocks);
        Err(DeviceError::NotSupported)
    }
}

pub trait CharDeviceOps: SharedDeviceOps {
    fn read(&self, buf: &mut [u8]) -> Result<usize, DeviceError>;
    fn write(&self, buf: &[u8]) -> Result<usize, DeviceError>;

    fn peek(&self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let _ = buf;
        Err(DeviceError::NotSupported)
    }

    fn has_data(&self) -> bool {
        false
    }

    fn has_space(&self) -> bool {
        false
    }

    // 新增非阻塞操作支持
    fn set_nonblocking(&self, nonblocking: bool) -> Result<(), DeviceError> {
        let _ = nonblocking;
        Err(DeviceError::NotSupported)
    }
}

#[derive(Clone)]
pub enum DeviceInner {
    Block(Arc<dyn BlockDeviceOps>),
    Char(Arc<dyn CharDeviceOps>),
    Network, // 为未来扩展预留
}

pub struct Device {
    pub name: String,
    pub major: u16,
    pub minor: u16,
    inner: DeviceInner,
    open_count: AtomicUsize,
    is_registered: bool, // 新增：跟踪设备注册状态
}

impl Device {
    pub fn new(name: String, major: u16, minor: u16, inner: DeviceInner) -> Self {
        Self {
            name,
            major,
            minor,
            inner,
            open_count: AtomicUsize::new(0),
            is_registered: false,
        }
    }

    pub fn new_auto_assign(name: String, inner: DeviceInner) -> Self {
        Self {
            name,
            major: 0,
            minor: 0,
            inner,
            open_count: AtomicUsize::new(0),
            is_registered: false,
        }
    }

    #[inline]
    fn shared_ops(&self) -> &dyn SharedDeviceOps {
        match &self.inner {
            DeviceInner::Block(ops) => ops.as_ref(),
            DeviceInner::Char(ops) => ops.as_ref(),
            _ => unimplemented!(),
        }
    }

    pub fn device_type(&self) -> DeviceType {
        self.shared_ops().device_type()
    }

    pub fn open(&self) -> Result<(), DeviceError> {
        if !self.is_registered {
            return Err(DeviceError::DeviceNotRegistered);
        }

        let current_count = self.open_count.fetch_add(1, Ordering::SeqCst);
        if current_count == 0 {
            self.shared_ops().open()?;
        }
        Ok(())
    }

    pub fn close(&self) -> Result<(), DeviceError> {
        if !self.is_registered {
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

    pub fn ioctl(&self, cmd: u64, arg: u64) -> Result<u64, DeviceError> {
        if !self.is_registered {
            return Err(DeviceError::DeviceNotRegistered);
        }
        if self.open_count.load(Ordering::SeqCst) == 0 {
            return Err(DeviceError::DeviceClosed);
        }
        self.shared_ops().ioctl(cmd, arg)
    }

    pub fn is_open(&self) -> bool {
        self.open_count.load(Ordering::Relaxed) > 0
    }

    pub fn as_block_device(&self) -> Option<&Arc<dyn BlockDeviceOps>> {
        if let DeviceInner::Block(ref ops) = self.inner {
            Some(ops)
        } else {
            None
        }
    }

    pub fn as_char_device(&self) -> Option<&Arc<dyn CharDeviceOps>> {
        if let DeviceInner::Char(ref ops) = self.inner {
            Some(ops)
        } else {
            None
        }
    }

    // 新增：标记注册状态
    fn mark_registered(&mut self) {
        self.is_registered = true;
    }

    // 新增：取消注册状态
    fn mark_unregistered(&mut self) {
        self.is_registered = false;
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
            self.is_registered
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
            is_registered: self.is_registered,
            inner: self.inner.clone(),
        }
    }
}

pub struct DeviceManager {
    devices: Vec<Device>,
    next_minor_counters: BTreeMap<u16, u16>,
    free_minors: BTreeMap<u16, Vec<u16>>, // 新增：空闲次设备号回收
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            next_minor_counters: BTreeMap::new(),
            free_minors: BTreeMap::new(), // 初始化空闲次设备号映射
        }
    }

    pub fn register_device(&mut self, mut device: Device) -> Result<(), DeviceError> {
        // 检查名称冲突
        if self.devices.iter().any(|d| d.name == device.name) {
            return Err(DeviceError::DeviceAlreadyRegistered);
        }

        // 自动分配设备号
        if device.major == 0 && device.minor == 0 {
            let (major, minor) = self.alloc_device_number(device.device_type())?;
            device.major = major;
            device.minor = minor;
        }
        // 手动分配设备号
        else {
            // 检查设备号冲突
            if self
                .devices
                .iter()
                .any(|d| d.major == device.major && d.minor == device.minor)
            {
                return Err(DeviceError::DeviceNumberConflict);
            }

            // 更新该主设备号的次设备号追踪器
            self.update_minor_counter(device.major, device.minor);
        }

        device.mark_registered(); // 标记为已注册
        self.devices.push(device);
        Ok(())
    }

    // 重构设备号分配函数
    fn alloc_device_number(&mut self, device_type: DeviceType) -> Result<(u16, u16), DeviceError> {
        let major = match device_type {
            DeviceType::Char => 1,
            DeviceType::Block => 2,
        };

        // 优先尝试从回收的次设备号中分配
        if let Some(minor) = self.free_minors.get_mut(&major).and_then(|v| v.pop()) {
            return Ok((major, minor));
        }

        // 从计数器中分配新次设备号
        let next_minor = self.next_minor_counters.entry(major).or_insert(0);
        let mut current_minor = *next_minor;

        // 查找可用次设备号 (最多尝试65535次)
        for _ in 0..u16::MAX as usize {
            // 将 is_minor_used 的逻辑内联以避免同时持有可变和不可变借用
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

    pub fn is_minor_used(&self, major: u16, minor: u16) -> bool {
        self.devices
            .iter()
            .any(|d| d.major == major && d.minor == minor)
    }

    fn update_minor_counter(&mut self, major: u16, minor: u16) {
        let counter = self.next_minor_counters.entry(major).or_insert(0);
        if minor >= *counter {
            *counter = minor + 1;
        }
    }

    pub fn unregister_device(&mut self, name: &str) -> Result<(), DeviceError> {
        let position = self.devices.iter().position(|d| d.name == name);

        if let Some(index) = position {
            if self.devices[index].is_open() {
                return Err(DeviceError::DeviceStillInUse);
            }

            let mut device = self.devices.remove(index);
            device.mark_unregistered(); // 更新注册状态

            // 回收设备号
            self.reclaim_device_number(device.major, device.minor);
            Ok(())
        } else {
            Err(DeviceError::NoSuchDevice)
        }
    }

    // 新增设备号回收方法
    fn reclaim_device_number(&mut self, major: u16, minor: u16) {
        self.free_minors
            .entry(major)
            .or_insert_with(Vec::new)
            .push(minor);
    }

    // 其他方法保持不变
    pub fn get_device(&self, name: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.name == name)
    }

    pub fn get_device_by_major_minor(&self, major: u16, minor: u16) -> Option<&Device> {
        self.devices
            .iter()
            .find(|d| d.major == major && d.minor == minor)
    }

    pub fn get_devices_by_type(&self, device_type: DeviceType) -> Vec<&Device> {
        self.devices
            .iter()
            .filter(|d| d.device_type() == device_type)
            .collect()
    }

    pub fn list_devices(&self) -> Vec<&Device> {
        self.devices.iter().collect()
    }
}

pub fn init_devices() {
    DEVICE_MANAGER
        .lock()
        .register_device(super::char::serial::SerialDevice::create_device(
            1, 0, 0x3f8,
        ))
        .expect("Failed to register serial device");
}

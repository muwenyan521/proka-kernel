extern crate alloc;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering}; // For open count
use lazy_static::lazy_static;
use spin::Mutex;

// 用于自动分配设备号的简单计数器（实际操作系统会更复杂）
// static NEXT_MAJOR: AtomicUsize = AtomicUsize::new(1);
// static NEXT_MINOR: AtomicUsize = AtomicUsize::new(0);

lazy_static! {
    pub static ref DEVICE_MANAGER: Mutex<DeviceManager> = Mutex::new(DeviceManager::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Block,
    Char,
    // Future: Network, Input, Pseudo, etc.
}

/// 更详细的设备错误类型
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DeviceError {
    InvalidParam,
    NotSupported,
    IoError,
    PermissionsDenied,
    NoSuchDevice,
    WouldBlock, // Non-blocking operation would block
    Busy,       // Device is currently busy
    OutOfMemory,
    DeviceClosed,
    BufferTooSmall,
    AlreadyOpen,
    NotOpen,
    AddressOutOfRange,
}

/// 所有设备类型通用的操作
pub trait SharedDeviceOps: Send + Sync {
    /// 获取设备名称。
    fn name(&self) -> &str;

    /// 获取设备的逻辑类型（块设备或字符设备）。
    fn device_type(&self) -> DeviceType;

    /// 打开设备。管理内部打开计数或初始化资源。
    fn open(&self) -> Result<(), DeviceError>;

    /// 关闭设备。减少内部打开计数或释放资源。
    fn close(&self) -> Result<(), DeviceError>;

    /// 执行设备特定的控制操作。
    /// `cmd` 是命令代码，`arg` 是参数（可以是值或指向数据的指针）。
    /// 返回一个 `u64` 结果或错误。
    fn ioctl(&self, cmd: u64, arg: u64) -> Result<u64, DeviceError>;
}

/// 块设备特有的操作
pub trait BlockDeviceOps: SharedDeviceOps {
    /// 返回设备的逻辑块大小（字节）。
    fn block_size(&self) -> usize;

    /// 返回设备上的总块数。
    fn num_blocks(&self) -> usize;

    /// 从 `block_idx` 读取 `num_blocks` 到 `buf`。
    /// `buf` 的长度必须是 `num_blocks * block_size()`。
    fn read_blocks(
        &self,
        block_idx: usize,
        num_blocks: usize,
        buf: &mut [u8],
    ) -> Result<usize, DeviceError>;

    /// 从 `buf` 写入 `num_blocks` 到 `block_idx`。
    /// `buf` 的长度必须是 `num_blocks * block_size()`。
    fn write_blocks(
        &self,
        block_idx: usize,
        num_blocks: usize,
        buf: &[u8],
    ) -> Result<usize, DeviceError>;
}

/// 字符设备特有的操作
pub trait CharDeviceOps: SharedDeviceOps {
    /// 从设备读取字节到 `buf`。对于字符设备，通常是流式的，不使用偏移量。
    /// 返回实际读取的字节数。
    fn read(&self, buf: &mut [u8]) -> Result<usize, DeviceError>;

    /// 从 `buf` 写入字节到设备。对于字符设备，通常是流式的，不使用偏移量。
    /// 返回实际写入的字节数。
    fn write(&self, buf: &[u8]) -> Result<usize, DeviceError>;

    /// 窥视（非消耗性读取）设备中的字节到 `buf`。
    /// 默认实现返回 `NotSupported`。
    fn peek(&self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let _ = buf; // 避免未使用的警告
        Err(DeviceError::NotSupported)
    }

    /// 检查是否有数据可供读取。
    /// 默认实现返回 `false`。
    fn has_data(&self) -> bool {
        false
    }

    /// 检查是否有空间可供写入。
    /// 默认实现返回 `false`。
    fn has_space(&self) -> bool {
        false
    }
}

/// 代表设备具体实现操作的枚举。
pub enum DeviceInner {
    Block(Arc<dyn BlockDeviceOps>),
    Char(Arc<dyn CharDeviceOps>),
}

/// 公共设备结构体，包含通用元数据和具体实现。
pub struct Device {
    pub name: String,
    pub major: u16, // 主设备号 (如：标识设备类型或控制器)
    pub minor: u16, // 次设备号 (如：标识特定设备实例)
    inner: DeviceInner,
    open_count: AtomicUsize, // 内部打开计数
}

impl Device {
    /// 构造一个新的设备。调用者提供其具体实现 (DeviceInner)。
    pub fn new(name: String, major: u16, minor: u16, inner: DeviceInner) -> Self {
        Self {
            name,
            major,
            minor,
            inner,
            open_count: AtomicUsize::new(0),
        }
    }

    /// 获取底层 SharedDeviceOps trait 对象的引用（用于通用操作）。
    #[inline]
    fn shared_ops(&self) -> &dyn SharedDeviceOps {
        match &self.inner {
            DeviceInner::Block(ops) => ops.as_ref(), // 将 Arc<dyn Trait> 转换为 &dyn Trait
            DeviceInner::Char(ops) => ops.as_ref(),
        }
    }

    // --- 通用操作的委托方法 ---
    /// 获取设备的逻辑类型。
    pub fn device_type(&self) -> DeviceType {
        self.shared_ops().device_type()
    }

    /// 打开设备。
    pub fn open(&self) -> Result<(), DeviceError> {
        let current_count = self.open_count.fetch_add(1, Ordering::SeqCst);
        if current_count == 0 {
            // 如果是第一次打开，则调用驱动的 open 方法
            self.shared_ops().open()?;
        }
        Ok(())
    }

    /// 关闭设备。
    pub fn close(&self) -> Result<(), DeviceError> {
        let current_count = self.open_count.fetch_sub(1, Ordering::SeqCst);
        if current_count == 1 {
            // 如果是最后一次关闭，则调用驱动的 close 方法
            self.shared_ops().close()?;
        } else if current_count == 0 {
            // 尝试关闭一个未打开的设备
            return Err(DeviceError::NotOpen);
        }
        Ok(())
    }

    /// 执行设备特定的控制操作。
    pub fn ioctl(&self, cmd: u64, arg: u64) -> Result<u64, DeviceError> {
        if self.open_count.load(Ordering::SeqCst) == 0 {
            return Err(DeviceError::DeviceClosed); // 必须先打开设备
        }
        self.shared_ops().ioctl(cmd, arg)
    }

    // --- 类型特定访问器 ---
    /// 如果设备是块设备，返回对其 `BlockDeviceOps` 实现的引用。
    pub fn as_block_device(&self) -> Option<&Arc<dyn BlockDeviceOps>> {
        if let DeviceInner::Block(ref ops) = self.inner {
            Some(ops)
        } else {
            None
        }
    }

    /// 如果设备是字符设备，返回对其 `CharDeviceOps` 实现的引用。
    pub fn as_char_device(&self) -> Option<&Arc<dyn CharDeviceOps>> {
        if let DeviceInner::Char(ref ops) = self.inner {
            Some(ops)
        } else {
            None
        }
    }
}

pub struct DeviceManager {
    devices: Vec<Device>,
    // 跟踪已分配的 major/minor 号，避免冲突
    major_minor_map: Vec<(u16, u16)>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            major_minor_map: Vec::new(),
        }
    }

    /// 注册一个设备。将检查设备名称和 major/minor 号的唯一性。
    pub fn register_device(&mut self, device: Device) -> Result<(), DeviceError> {
        // 检查名称是否重复
        if self.devices.iter().any(|d| d.name == device.name) {
            panic!("Device {} already exists", device.name);
        }

        // 检查 major/minor 是否重复
        if self.major_minor_map.contains(&(device.major, device.minor)) {
            panic!(
                "Device with major:{} minor:{} already exists",
                device.major, device.minor
            );
        }

        self.major_minor_map.push((device.major, device.minor));
        self.devices.push(device);
        Ok(())
    }

    /// 根据设备名称获取设备。
    pub fn get_device(&self, name: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.name == name)
    }

    /// 根据 major 和 minor 号获取设备。
    pub fn get_device_by_major_minor(&self, major: u16, minor: u16) -> Option<&Device> {
        self.devices
            .iter()
            .find(|d| d.major == major && d.minor == minor)
    }

    /// 根据设备类型获取所有匹配的设备。
    pub fn get_devices_by_type(&self, device_type: DeviceType) -> Vec<&Device> {
        self.devices
            .iter()
            .filter(|d| d.device_type() == device_type)
            .collect()
    }

    /// 注销设备。
    pub fn unregister_device(&mut self, name: &str) -> bool {
        if let Some(index) = self.devices.iter().position(|d| d.name == name) {
            let removed_device = self.devices.remove(index);
            self.major_minor_map
                .retain(|&(maj, min)| maj != removed_device.major || min != removed_device.minor);
            return true;
        }
        false
    }
}

//! 串口设备驱动程序
//!
//! 此模块提供了基于16550 UART兼容串口的字符设备驱动程序。
//! 它实现了 [`CharDevice`] trait，允许通过串口进行字符输入/输出操作。
//!
//! # 功能
//!
//! - 支持标准COM端口（COM1-COM4）
//! - 线程安全的串口访问（使用 [`RwLock`] 保护）
//! - 自动设备号分配支持
//! - 基本的I/O控制（ioctl）操作
//!
//! # 示例
//!
//! ```rust
//! // 创建COM1串口设备（端口地址0x3F8）
//! let com1 = SerialDevice::new(0x3F8);
//!
//! // 创建带有指定主/次设备号的设备实例
//! let device = SerialDevice::create_device(4, 64, 0x3F8);
//!
//! // 使用自动分配的设备号创建设备
//! let auto_device = SerialDevice::create_device_auto_assign(0x3F8);
//! ```
//!
//! # 端口地址
//!
//! 常见的串口端口地址：
//! - COM1: 0x3F8
//! - COM2: 0x2F8
//! - COM3: 0x3E8
//! - COM4: 0x2E8
//!
//! # 注意
//!
//! 此实现目前仅支持输出操作（`write`），输入操作（`read`）返回 [`DeviceError::NotSupported`]。
//! 这是因为在早期启动阶段，串口通常用于调试输出而非输入。

extern crate alloc;

use super::super::{CharDevice, Device, DeviceError, DeviceInner, DeviceType, SharedDeviceOps};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use spin::RwLock;
use uart_16550::SerialPort;

/// 串口设备结构体
///
/// 表示一个16550 UART兼容的串口设备，提供字符设备接口。
///
/// # 字段
///
/// - `port_address`: 串口的I/O端口地址（例如0x3F8对应COM1）
/// - `name`: 设备名称，格式为"serial-{port_address}"
/// - `serial_port`: 受读写锁保护的串口实例，确保线程安全访问
///
/// # 线程安全
///
/// 使用 [`RwLock`] 保护串口访问，允许多个读取者或单个写入者同时访问。
/// 这对于内核中的并发访问是必要的。
pub struct SerialDevice {
    port_address: u16,
    name: String,
    serial_port: RwLock<SerialPort>,
}

impl SerialDevice {
    /// 创建一个新的串口设备实例
    ///
    /// # 参数
    ///
    /// - `port_address`: 串口的I/O端口地址（例如0x3F8对应COM1）
    ///
    /// # 返回
    ///
    /// 返回初始化的 [`SerialDevice`] 实例
    ///
    /// # 安全性
    ///
    /// 此函数使用 `unsafe` 块创建 [`SerialPort`] 实例，因为直接访问I/O端口
    /// 需要确保端口地址有效且未被其他设备使用。
    ///
    /// # 示例
    ///
    /// ```rust
    /// let com1 = SerialDevice::new(0x3F8); // COM1
    /// ```
    pub fn new(port_address: u16) -> Self {
        let mut serial_port = unsafe { SerialPort::new(port_address) };
        serial_port.init();
        Self {
            port_address,
            name: format!("serial-{}", port_address),
            serial_port: RwLock::new(serial_port),
        }
    }

    /// 创建一个串口字符设备实例，并封装为通用的 `Device` 结构
    ///
    /// 此方法允许用户手动指定主设备号和次设备号。
    ///
    /// # 参数
    ///
    /// - `major`: 主设备号（通常4表示tty设备）
    /// - `minor`: 次设备号（64-255范围内的值）
    /// - `port_address`: 串口的I/O端口地址
    ///
    /// # 返回
    ///
    /// 返回封装好的 [`Device`] 实例，可以注册到设备管理器中
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 创建COM1设备，主设备号4，次设备号64
    /// let device = SerialDevice::create_device(4, 64, 0x3F8);
    /// ```
    pub fn create_device(major: u16, minor: u16, port_address: u16) -> Device {
        let serial = Arc::new(SerialDevice::new(port_address));
        Device::new(
            serial.name().to_string(),
            major,
            minor,
            DeviceInner::Char(serial),
        )
    }

    /// 创建一个串口字符设备实例，并让 `DeviceManager` 自动分配主/次设备号
    ///
    /// 此方法使用 [`Device::new_auto_assign`] 来自动分配设备号，
    /// 简化了设备创建过程。
    ///
    /// # 参数
    ///
    /// - `port_address`: 串口的I/O端口地址
    ///
    /// # 返回
    ///
    /// 返回封装好的 [`Device`] 实例，带有自动分配的设备号
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 创建COM1设备，自动分配设备号
    /// let device = SerialDevice::create_device_auto_assign(0x3F8);
    /// ```
    pub fn create_device_auto_assign(port_address: u16) -> Device {
        let serial = Arc::new(SerialDevice::new(port_address));
        Device::new_auto_assign(serial.name().to_string(), DeviceInner::Char(serial))
    }
}

impl SharedDeviceOps for SerialDevice {
    /// 获取设备名称
    ///
    /// 返回格式为"serial-{port_address}"的设备名称字符串
    fn name(&self) -> &str {
        &self.name
    }

    /// 获取设备类型
    ///
    /// 串口设备始终返回 [`DeviceType::Char`]，表示字符设备
    fn device_type(&self) -> DeviceType {
        DeviceType::Char
    }

    /// 打开设备
    ///
    /// 对于串口设备，打开操作总是成功，因为串口在创建时已经初始化
    ///
    /// # 返回
    ///
    /// 总是返回 `Ok(())`
    fn open(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// 关闭设备
    ///
    /// 对于串口设备，关闭操作总是成功
    ///
    /// # 返回
    ///
    /// 总是返回 `Ok(())`
    fn close(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// 执行I/O控制操作
    ///
    /// 支持的命令：
    /// - `cmd = 1`: 获取端口地址，返回端口地址作为 `u64`
    ///
    /// # 参数
    ///
    /// - `cmd`: 控制命令
    /// - `_arg`: 命令参数（当前未使用）
    ///
    /// # 返回
    ///
    /// - 对于命令1：返回端口地址
    /// - 对于其他命令：返回 [`DeviceError::NotSupported`]
    ///
    /// # 示例
    ///
    /// ```rust
    /// let com1 = SerialDevice::new(0x3F8);
    /// let port = com1.ioctl(1, 0).unwrap(); // 返回0x3F8
    /// ```
    fn ioctl(&self, cmd: u64, _arg: u64) -> Result<u64, DeviceError> {
        match cmd {
            1 => Ok(self.port_address as u64),
            _ => Err(DeviceError::NotSupported),
        }
    }
}

impl CharDevice for SerialDevice {
    /// 从设备读取数据
    ///
    /// # 注意
    ///
    /// 当前实现不支持读取操作，总是返回 [`DeviceError::NotSupported`]。
    /// 这是因为在早期启动阶段，串口主要用于调试输出。
    ///
    /// # 参数
    ///
    /// - `_buf`: 读取缓冲区（当前未使用）
    ///
    /// # 返回
    ///
    /// 总是返回 [`DeviceError::NotSupported`]
    fn read(&self, _buf: &mut [u8]) -> Result<usize, DeviceError> {
        Err(DeviceError::NotSupported)
    }

    /// 向设备写入数据
    ///
    /// 将缓冲区中的数据写入串口，逐个字节发送。
    ///
    /// # 参数
    ///
    /// - `buf`: 要写入的数据缓冲区
    ///
    /// # 返回
    ///
    /// 返回成功写入的字节数（与缓冲区长度相同）
    ///
    /// # 示例
    ///
    /// ```rust
    /// let com1 = SerialDevice::new(0x3F8);
    /// let data = b"Hello, World!";
    /// let written = com1.write(data).unwrap(); // 返回13
    /// ```
    fn write(&self, buf: &[u8]) -> Result<usize, DeviceError> {
        let mut serial_port = self.serial_port.write();
        for byte in buf {
            serial_port.send(*byte);
        }
        Ok(buf.len())
    }
}

extern crate alloc;

use super::super::{CharDevice, Device, DeviceError, DeviceInner, DeviceType, SharedDeviceOps};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use spin::RwLock;
use uart_16550::SerialPort;

pub struct SerialDevice {
    port_address: u16,
    name: String,
    serial_port: RwLock<SerialPort>,
}

impl SerialDevice {
    pub fn new(port_address: u16) -> Self {
        let mut serial_port = unsafe { SerialPort::new(port_address) };
        serial_port.init();
        Self {
            port_address,
            name: format!("serial-{}", port_address),
            serial_port: RwLock::new(serial_port),
        }
    }

    /// 创建一个串口字符设备实例，并封装为通用的 `Device` 结构。
    /// 用户需要手动指定 major/minor 号。
    pub fn create_device(major: u16, minor: u16, port_address: u16) -> Device {
        let serial = Arc::new(SerialDevice::new(port_address));
        Device::new(
            serial.name().to_string(),
            major,
            minor,
            DeviceInner::Char(serial),
        )
    }

    /// 创建一个串口字符设备实例，并让 `DeviceManager` 自动分配 major/minor 号。
    pub fn create_device_auto_assign(port_address: u16) -> Device {
        let serial = Arc::new(SerialDevice::new(port_address));
        Device::new_auto_assign(serial.name().to_string(), DeviceInner::Char(serial))
    }
}

impl SharedDeviceOps for SerialDevice {
    fn name(&self) -> &str {
        &self.name
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Char
    }

    fn open(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn close(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    fn ioctl(&self, cmd: u64, _arg: u64) -> Result<u64, DeviceError> {
        match cmd {
            1 => Ok(self.port_address as u64),
            _ => Err(DeviceError::NotSupported),
        }
    }
}

impl CharDevice for SerialDevice {
    fn read(&self, _buf: &mut [u8]) -> Result<usize, DeviceError> {
        Err(DeviceError::NotSupported)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, DeviceError> {
        let mut serial_port = self.serial_port.write();
        for byte in buf {
            serial_port.send(*byte);
        }
        Ok(buf.len())
    }
}

extern crate alloc;
use super::super::{Device, DeviceError, DeviceOps, DeviceType};
use alloc::format;
use alloc::sync::Arc;
use spin::RwLock;
use uart_16550::SerialPort;

pub struct SerialDevice {
    serial_port: RwLock<SerialPort>,
}

impl SerialDevice {
    pub fn new(port: u16) -> Self {
        let mut serial_port = unsafe { SerialPort::new(port) };
        serial_port.init();
        Self {
            serial_port: RwLock::new(serial_port),
        }
    }

    pub fn create_device(port: u16) -> Device {
        Device {
            device_type: DeviceType::Char,
            name: format!("serial-{}", port),
            ops: Arc::new(SerialDevice::new(port)),
        }
    }
}

impl DeviceOps for SerialDevice {
    fn device_type(&self) -> DeviceType {
        DeviceType::Char
    }
    fn read(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize, DeviceError> {
        Err(DeviceError::NotSupported)
    }
    fn write(&self, _offset: usize, buf: &[u8]) -> Result<usize, DeviceError> {
        let mut serial_port = self.serial_port.write();
        for byte in buf {
            serial_port.send(*byte);
        }
        Ok(buf.len())
    }
    fn ioctl(&self, _cmd: usize, _arg: usize) -> Result<usize, DeviceError> {
        Err(DeviceError::NotSupported)
    }
}

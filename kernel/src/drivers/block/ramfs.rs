extern crate alloc;
use super::super::{Device, DeviceError, DeviceOps, DeviceType};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;

pub struct MemDevice {
    data: Mutex<Vec<u8>>,
}

impl MemDevice {
    pub fn new(size: usize) -> Self {
        Self {
            data: Mutex::new(vec![0; size]),
        }
    }

    pub fn create_device() -> Device {
        Device {
            device_type: DeviceType::Char,
            name: "mem".into(),
            ops: Arc::new(MemDevice::new(1024)), // 1KB内存设备
        }
    }
}

impl DeviceOps for MemDevice {
    fn device_type(&self) -> DeviceType {
        DeviceType::Char
    }

    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let data = self.data.lock();
        if offset >= data.len() {
            return Err(DeviceError::InvalidParam);
        }
        let len = buf.len().min(data.len() - offset);
        buf[..len].copy_from_slice(&data[offset..offset + len]);
        Ok(len)
    }

    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize, DeviceError> {
        let mut data = self.data.lock();
        if offset >= data.len() {
            return Err(DeviceError::InvalidParam);
        }
        let len = buf.len().min(data.len() - offset);
        data[offset..offset + len].copy_from_slice(&buf[..len]);
        Ok(len)
    }

    fn ioctl(&self, _cmd: usize, _arg: usize) -> Result<usize, DeviceError> {
        Ok(0) // 简单实现
    }
}

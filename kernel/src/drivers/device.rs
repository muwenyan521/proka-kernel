extern crate alloc;
use alloc::string::String;
use alloc::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Block,
    Char,
}

pub trait DeviceOps: Send + Sync {
    fn device_type(&self) -> DeviceType;
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, ()>;
    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize, ()>;
    fn ioctl(&self, cmd: usize, arg: usize) -> Result<usize, ()>;
}

pub struct Device {
    pub device_type: DeviceType,
    pub name: String, // 设备名
    pub ops: Arc<dyn DeviceOps>,
}

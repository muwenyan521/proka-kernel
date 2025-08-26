extern crate alloc;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref DEVICE_MANAGER: Mutex<DeviceManager> = Mutex::new(DeviceManager::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Block,
    Char,
}

pub trait DeviceOps: Send + Sync {
    fn device_type(&self) -> DeviceType;
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, DeviceError>;
    fn write(&self, offset: usize, buf: &[u8]) -> Result<usize, DeviceError>;
    fn ioctl(&self, cmd: usize, arg: usize) -> Result<usize, DeviceError>;
}

pub struct Device {
    pub device_type: DeviceType,
    pub name: String, // 设备名
    pub ops: Arc<dyn DeviceOps>,
}

#[derive(Debug)]
pub enum DeviceError {
    InvalidParam,
    NotSupported,
    IoError,
}

pub struct DeviceManager {
    devices: Vec<Device>,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    pub fn register_device(&mut self, device: Device) {
        if self.devices.iter().any(|d| d.name == device.name) {
            panic!("Device {} already exists", device.name);
        }
        self.devices.push(device);
    }

    pub fn get_device(&self, name: &str) -> Option<&Device> {
        for device in self.devices.iter() {
            if device.name == name {
                return Some(device);
            }
        }
        None
    }

    pub fn get_device_by_type(&self, device_type: DeviceType) -> Option<&Device> {
        self.devices
            .iter()
            .find(|device| device.device_type == device_type)
    }

    pub fn unregister_device(&mut self, name: &str) -> bool {
        for (index, device) in self.devices.iter().enumerate() {
            if device.name == name {
                self.devices.remove(index);
                return true;
            }
        }
        false
    }
}

extern crate alloc;
pub mod device;
use alloc::vec::Vec;
use device::{Device, DeviceType};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref DEVICE_MANAGER: Mutex<DeviceManager> = Mutex::new(DeviceManager::new());
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

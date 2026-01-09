use super::BlockDevice;
extern crate alloc;
use crate::drivers::DeviceError;
use alloc::vec;
use alloc::vec::Vec;
use spin::RwLock;

#[allow(dead_code)]
pub struct RamBlockDevice {
    storage: RwLock<Vec<u8>>,
    block_size: usize,
}

impl RamBlockDevice {
    #[allow(dead_code)]
    pub fn new(num_blocks: usize, block_size: usize) -> Self {
        Self {
            storage: RwLock::new(vec![0; num_blocks * block_size]),
            block_size,
        }
    }
}

impl BlockDevice for RamBlockDevice {
    fn block_size(&self) -> usize {
        self.block_size
    }
    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> Result<usize, DeviceError> {
        let start = block_id * self.block_size;
        let end = start + self.block_size;
        let storage = self.storage.read();
        if end > storage.len() {
            return Err(DeviceError::AddressOutOfRange);
        }
        buf.copy_from_slice(&storage[start..end]);
        Ok(self.block_size)
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) -> Result<usize, DeviceError> {
        let start = block_id * self.block_size;
        let end = start + self.block_size;
        let mut storage = self.storage.write();
        if end > storage.len() {
            return Err(DeviceError::AddressOutOfRange);
        }
        storage[start..end].copy_from_slice(buf);
        Ok(self.block_size)
    }
}

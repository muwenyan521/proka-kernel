mod ramblk;

extern crate alloc;
use crate::drivers::DeviceError;
pub trait BlockDevice: Send + Sync {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> Result<usize, DeviceError>;

    fn write_block(&self, block_id: usize, buf: &[u8]) -> Result<usize, DeviceError>;

    fn block_size(&self) -> usize {
        512
    }
}

pub trait BlockOperations: Send + Sync {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, DeviceError>;
    fn write_at(&self, offset: u64, buf: &[u8]) -> Result<usize, DeviceError>;
    fn flush(&self) -> Result<(), DeviceError>;
}

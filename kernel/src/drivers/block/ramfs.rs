extern crate alloc;
use super::super::{BlockDeviceOps, Device, DeviceError, DeviceInner, DeviceType, SharedDeviceOps};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex; // For open count in the device driver itself

/// 块设备默认的块大小
const RAMFS_BLOCK_SIZE: usize = 512;

pub struct RamFSDevice {
    #[allow(dead_code)]
    id: u16,
    name: String,
    data: Mutex<Vec<u8>>,
    block_size: usize,
    num_blocks: usize,
    // open_count: AtomicUsize, // Device struct now handles this
}

impl RamFSDevice {
    pub fn new(id: u16, size_bytes: usize) -> Self {
        let block_size = RAMFS_BLOCK_SIZE;
        let num_blocks = (size_bytes + block_size - 1) / block_size; // 向上取整
        let actual_size_bytes = num_blocks * block_size;
        Self {
            id,
            name: format!("ramfs-{}", id),
            data: Mutex::new(vec![0; actual_size_bytes]),
            block_size,
            num_blocks,
            // open_count: AtomicUsize::new(0),
        }
    }

    /// 创建一个 RamFS 块设备实例，并封装为通用的 `Device` 结构。
    /// major/minor 号通常由设备管理器或驱动框架分配。
    pub fn create_device(id: u16, major: u16, minor: u16, size_bytes: usize) -> Device {
        let ramfs = Arc::new(RamFSDevice::new(id, size_bytes));
        Device::new(
            ramfs.name().to_string(),
            major,
            minor,
            DeviceInner::Block(ramfs),
        )
    }
}

impl SharedDeviceOps for RamFSDevice {
    fn name(&self) -> &str {
        &self.name
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn open(&self) -> Result<(), DeviceError> {
        // 在这里可以执行 RamFS 特有的初始化操作，比如检查状态。
        // 设备结构的 open_count 已经处理了重复打开的逻辑。
        // println!("RamFS device {} opened", self.name); // for debugging
        Ok(())
    }

    fn close(&self) -> Result<(), DeviceError> {
        // 在这里可以执行 RamFS 特有的清理操作。
        // println!("RamFS device {} closed", self.name); // for debugging
        Ok(())
    }

    fn ioctl(&self, _cmd: u64, _arg: u64) -> Result<u64, DeviceError> {
        Err(DeviceError::NotSupported)
    }
}

impl BlockDeviceOps for RamFSDevice {
    fn block_size(&self) -> usize {
        self.block_size
    }

    fn num_blocks(&self) -> usize {
        self.num_blocks
    }

    fn read_blocks(
        &self,
        block_idx: usize,
        num_blocks: usize,
        buf: &mut [u8],
    ) -> Result<usize, DeviceError> {
        let data = self.data.lock();
        let requested_len = num_blocks * self.block_size;

        if block_idx + num_blocks > self.num_blocks {
            return Err(DeviceError::InvalidParam);
        }
        if buf.len() < requested_len {
            return Err(DeviceError::BufferTooSmall);
        }

        let offset = block_idx * self.block_size;
        let end = offset + requested_len;

        if end > data.len() {
            // 这应该被上面的 block_idx + num_blocks > self.num_blocks 捕获
            return Err(DeviceError::AddressOutOfRange);
        }

        buf[..requested_len].copy_from_slice(&data[offset..end]);
        Ok(requested_len)
    }

    fn write_blocks(
        &self,
        block_idx: usize,
        num_blocks: usize,
        buf: &[u8],
    ) -> Result<usize, DeviceError> {
        let mut data = self.data.lock();
        let requested_len = num_blocks * self.block_size;

        if block_idx + num_blocks > self.num_blocks {
            return Err(DeviceError::InvalidParam);
        }
        if buf.len() < requested_len {
            return Err(DeviceError::BufferTooSmall);
        }

        let offset = block_idx * self.block_size;
        let end = offset + requested_len;

        if end > data.len() {
            return Err(DeviceError::AddressOutOfRange);
        }

        data[offset..end].copy_from_slice(&buf[..requested_len]);
        Ok(requested_len)
    }
}

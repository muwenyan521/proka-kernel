extern crate alloc;
use crate::drivers::block::BlockDevice;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

pub struct BlockCache {
    device: Arc<dyn BlockDevice>,
    cache: RwLock<BTreeMap<usize, Vec<u8>>>,
}

impl BlockCache {
    pub fn new(device: Arc<dyn BlockDevice>) -> Self {
        Self {
            device,
            cache: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn read_block(&self, block_id: usize) -> Option<Vec<u8>> {
        {
            let cache = self.cache.read();
            if let Some(data) = cache.get(&block_id) {
                return Some(data.clone());
            }
        }

        let block_size = self.device.block_size();
        let mut buf = Vec::with_capacity(block_size);
        buf.resize(block_size, 0);

        if self.device.read_block(block_id, &mut buf).is_ok() {
            let mut cache = self.cache.write();
            cache.insert(block_id, buf.clone());
            Some(buf)
        } else {
            None
        }
    }

    pub fn write_block(&self, block_id: usize, data: &[u8]) -> bool {
        if self.device.write_block(block_id, data).is_ok() {
            let mut cache = self.cache.write();
            cache.insert(block_id, data.to_vec());
            true
        } else {
            false
        }
    }

    pub fn sync(&self) {}
}

extern crate alloc;
use crate::drivers::DeviceOps;
use crate::fs::vfs::{File, FileSystem, Metadata, VNode, VNodeType, VfsError};
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::Mutex;

// --- 内存文件系统实现 ---
struct MemFile {
    data: Mutex<Vec<u8>>,
    pos: Mutex<u64>,
    metadata: Metadata,
}

impl File for MemFile {
    fn read(&self, buf: &mut [u8]) -> Result<usize, VfsError> {
        let mut pos = self.pos.lock();
        let data = self.data.lock();
        let len = core::cmp::min(buf.len(), (data.len() as u64 - *pos) as usize);
        buf[..len].copy_from_slice(&data[*pos as usize..][..len]);
        *pos += len as u64;
        Ok(len)
    }

    fn write(&self, buf: &[u8]) -> Result<usize, VfsError> {
        let mut data = self.data.lock();
        let mut pos = self.pos.lock();
        let write_pos = *pos as usize;

        if write_pos + buf.len() > data.len() {
            data.resize(write_pos + buf.len(), 0);
        }

        data[write_pos..write_pos + buf.len()].copy_from_slice(buf);
        *pos += buf.len() as u64;
        Ok(buf.len())
    }

    fn seek(&self, pos: u64) -> Result<u64, VfsError> {
        let mut current = self.pos.lock();
        *current = pos;
        Ok(pos)
    }
}

impl VNode for MemFile {
    fn node_type(&self) -> VNodeType {
        VNodeType::File
    }

    fn metadata(&self) -> Result<Metadata, VfsError> {
        Ok(self.metadata.clone())
    }

    fn open(&self) -> Result<Box<dyn File>, VfsError> {
        Ok(Box::new(MemFile {
            data: Mutex::new(self.data.lock().clone()),
            pos: Mutex::new(0),
            metadata: self.metadata.clone(),
        }))
    }

    fn lookup(&self, _name: &str) -> Result<Arc<dyn VNode>, VfsError> {
        Err(VfsError::NotADirectory)
    }

    fn create(&self, _name: &str, _typ: VNodeType) -> Result<Arc<dyn VNode>, VfsError> {
        Err(VfsError::NotADirectory)
    }
}

struct MemDir {
    children: Mutex<BTreeMap<String, Arc<dyn VNode>>>,
    metadata: Metadata,
}

impl VNode for MemDir {
    fn node_type(&self) -> VNodeType {
        VNodeType::Dir
    }

    fn metadata(&self) -> Result<Metadata, VfsError> {
        Ok(self.metadata.clone())
    }

    fn open(&self) -> Result<Box<dyn File>, VfsError> {
        Err(VfsError::NotAFile)
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn VNode>, VfsError> {
        self.children
            .lock()
            .get(name)
            .cloned()
            .ok_or(VfsError::NotFound)
    }

    fn create(&self, name: &str, typ: VNodeType) -> Result<Arc<dyn VNode>, VfsError> {
        if self.children.lock().contains_key(name) {
            return Err(VfsError::AlreadyExists);
        }

        let node: Arc<dyn VNode> = match typ {
            VNodeType::File => Arc::new(MemFile {
                data: Mutex::new(Vec::new()),
                pos: Mutex::new(0),
                metadata: Metadata {
                    size: 0,
                    permissions: 0o644,
                    uid: 0,
                    gid: 0,
                    ctime: 0,
                    mtime: 0,
                },
            }),
            VNodeType::Dir => Arc::new(MemDir {
                children: Mutex::new(BTreeMap::new()),
                metadata: Metadata {
                    size: 0,
                    permissions: 0o755,
                    uid: 0,
                    gid: 0,
                    ctime: 0,
                    mtime: 0,
                },
            }),
            _ => return Err(VfsError::InvalidArgument),
        };

        self.children.lock().insert(name.to_string(), node.clone());
        Ok(node)
    }
}

pub struct MemFs;

impl FileSystem for MemFs {
    fn mount(&self, _device: Option<Arc<dyn DeviceOps>>) -> Result<Arc<dyn VNode>, VfsError> {
        Ok(Arc::new(MemDir {
            children: Mutex::new(BTreeMap::new()),
            metadata: Metadata {
                size: 0,
                permissions: 0o755,
                uid: 0,
                gid: 0,
                ctime: 0,
                mtime: 0,
            },
        }))
    }

    fn fs_type(&self) -> &'static str {
        "memfs"
    }
}

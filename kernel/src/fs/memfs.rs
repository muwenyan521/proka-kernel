extern crate alloc;
use super::vfs::{File, FileSystem, Metadata, VNode, VNodeType, VfsError};
use crate::{drivers::Device, println};
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use core::fmt;
use spin::{Mutex, RwLock};

#[derive(Debug, Clone)]
struct MemFile {
    data: Vec<u8>,
    metadata: Metadata,
}

impl File for MemFile {
    fn read(&self, buf: &mut [u8]) -> Result<usize, VfsError> {
        println!("{}", self.data.len());
        let len = core::cmp::min(buf.len(), self.data.len());
        buf[..len].copy_from_slice(&self.data[..len]);
        Ok(len)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, VfsError> {
        self.data.extend_from_slice(buf);
        self.metadata.size = self.data.len() as u64;
        Ok(buf.len())
    }

    fn seek(&self, _pos: u64) -> Result<u64, VfsError> {
        Ok(0) // 简化实现，不支持seek
    }

    fn stat(&self) -> Result<Metadata, VfsError> {
        Ok(self.metadata.clone())
    }
}

#[derive(Debug)]
struct MemVNode {
    name: String,
    typ: VNodeType,
    parent: Weak<MemVNode>,
    children: Mutex<BTreeMap<String, Arc<MemVNode>>>,
    file: RwLock<Option<MemFile>>,
    metadata: Metadata,
}

impl MemVNode {
    fn new(name: String, typ: VNodeType, parent: Weak<MemVNode>, metadata: Metadata) -> Arc<Self> {
        Arc::new(Self {
            name,
            typ,
            parent,
            children: Mutex::new(BTreeMap::new()),
            file: RwLock::new(if typ == VNodeType::File {
                Some(MemFile {
                    data: Vec::new(),
                    metadata: metadata.clone(),
                })
            } else {
                None
            }),
            metadata,
        })
    }
}

impl VNode for MemVNode {
    fn node_type(&self) -> VNodeType {
        self.typ
    }

    fn metadata(&self) -> Result<Metadata, VfsError> {
        Ok(self.metadata.clone())
    }

    fn open(&self) -> Result<Box<dyn File>, VfsError> {
        if self.typ != VNodeType::File {
            return Err(VfsError::NotAFile);
        }
        let file = self.file.read().clone();
        file.map(|f| Box::new(f) as Box<dyn File>)
            .ok_or(VfsError::IoError)
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn VNode>, VfsError> {
        if self.typ != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        self.children
            .lock()
            .get(name)
            .map(|n| n.clone() as Arc<dyn VNode>)
            .ok_or(VfsError::NotFound)
    }

    fn create(&self, name: &str, typ: VNodeType) -> Result<Arc<dyn VNode>, VfsError> {
        if self.typ != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }

        let mut children = self.children.lock();
        if children.contains_key(name) {
            return Err(VfsError::AlreadyExists);
        }

        let now = 0; // 在实际系统中应该是当前时间
        let metadata = Metadata {
            size: 0,
            permissions: 0o644,
            uid: 0,
            gid: 0,
            ctime: now,
            mtime: now,
        };

        let self_arc = unsafe {
            // 安全的因为我们知道self是MemVNode的一部分
            Arc::from_raw(self as *const MemVNode as *const MemVNode as *mut MemVNode)
        };
        let parent = Arc::downgrade(&self_arc);
        // 避免Arc被释放
        Arc::into_raw(self_arc);

        let node = MemVNode::new(name.to_string(), typ, parent, metadata);

        children.insert(name.to_string(), node.clone());
        Ok(node as Arc<dyn VNode>)
    }
}

pub struct MemFs;

impl FileSystem for MemFs {
    fn mount(
        &self,
        _device: Option<&Device>,
        _args: Option<&[&str]>,
    ) -> Result<Arc<dyn VNode>, VfsError> {
        let now = 0; // 在实际系统中应该是当前时间
        let metadata = Metadata {
            size: 0,
            permissions: 0o755,
            uid: 0,
            gid: 0,
            ctime: now,
            mtime: now,
        };

        Ok(MemVNode::new(
            "".to_string(),
            VNodeType::Dir,
            Weak::new(),
            metadata,
        ))
    }

    fn fs_type(&self) -> &'static str {
        "memfs"
    }
}

impl fmt::Debug for MemFs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemFs").finish()
    }
}

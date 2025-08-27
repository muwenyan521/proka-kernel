extern crate alloc;
use super::vfs::{File, FileSystem, Metadata, VNode, VNodeType, VfsError};
use crate::{drivers::Device, println}; // Keep println for debugging
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use core::fmt;
use spin::{Mutex, RwLock};

#[derive(Debug)]
struct MemFileContent {
    data: Vec<u8>,
    metadata: Metadata,
}

#[derive(Debug)]
struct MemFileHandle {
    content_arc: Arc<RwLock<MemFileContent>>,
}

impl File for MemFileHandle {
    fn read(&self, buf: &mut [u8]) -> Result<usize, VfsError> {
        let content = self.content_arc.read();
        println!("MemFileHandle::read - data.len(): {}", content.data.len()); // Debug print
        let len = core::cmp::min(buf.len(), content.data.len());
        buf[..len].copy_from_slice(&content.data[..len]);
        Ok(len)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, VfsError> {
        let mut content = self.content_arc.write();
        content.data.extend_from_slice(buf);
        content.metadata.size = content.data.len() as u64; // 更新文件大小
        Ok(buf.len())
    }

    fn seek(&self, _pos: u64) -> Result<u64, VfsError> {
        Ok(0) // 简化实现，不支持seek
    }

    fn stat(&self) -> Result<Metadata, VfsError> {
        let content = self.content_arc.read();
        Ok(content.metadata.clone())
    }
}

#[derive(Debug)]
struct MemVNode {
    #[allow(dead_code)]
    name: String,
    typ: VNodeType,
    #[allow(dead_code)]
    parent: Weak<MemVNode>,
    children: Mutex<BTreeMap<String, Arc<MemVNode>>>,
    // 对于文件，这里保存了对实际文件内容（MemFileContent）的共享引用。
    // 对于目录，这里是 None。
    file_content_arc: Option<Arc<RwLock<MemFileContent>>>,
    // 这个 Metadata 是当前 VNode 自身的元数据，例如目录的元数据。
    // 对于文件，VNode::metadata() 会从 file_content_arc 中获取。
    metadata: Metadata,
}

impl MemVNode {
    fn new(name: String, typ: VNodeType, parent: Weak<MemVNode>, metadata: Metadata) -> Arc<Self> {
        let file_content_arc = if typ == VNodeType::File {
            Some(Arc::new(RwLock::new(MemFileContent {
                data: Vec::new(),
                metadata: metadata.clone(), // 文件的初始元数据
            })))
        } else {
            None
        };

        Arc::new(Self {
            name,
            typ,
            parent,
            children: Mutex::new(BTreeMap::new()),
            file_content_arc,
            metadata, // VNode 自身的元数据
        })
    }
}

impl VNode for MemVNode {
    fn node_type(&self) -> VNodeType {
        self.typ
    }

    fn metadata(&self) -> Result<Metadata, VfsError> {
        match self.typ {
            VNodeType::File => {
                // 对于文件，从实际文件内容中获取元数据
                if let Some(content_arc) = self.file_content_arc.as_ref() {
                    let content = content_arc.read();
                    Ok(content.metadata.clone())
                } else {
                    Err(VfsError::IoError) // 文件 VNode 但没有内容，不应该发生
                }
            }
            _ => {
                // 对于目录、符号链接等，使用 VNode 自身的元数据
                Ok(self.metadata.clone())
            }
        }
    }

    fn open(&self) -> Result<Box<dyn File>, VfsError> {
        if self.typ != VNodeType::File {
            return Err(VfsError::NotAFile);
        }

        let content_arc = self
            .file_content_arc
            .as_ref()
            .ok_or(VfsError::IoError)?
            .clone(); // 克隆 Arc，获得对共享内容的引用

        Ok(Box::new(MemFileHandle { content_arc }))
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
            permissions: if typ == VNodeType::Dir { 0o755 } else { 0o644 }, // 根据类型设置默认权限
            uid: 0,
            gid: 0,
            ctime: now,
            mtime: now,
        };

        // 为了简化和避免 unsafe，这里创建的子节点父引用暂时为 Weak::new()。
        // 如果需要正确的父子链，VNode::create 方法的签名可能需要修改。
        let node = MemVNode::new(name.to_string(), typ, Weak::new(), metadata);

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

        // 根节点的父节点总是空的
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

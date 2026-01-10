//! 内存文件系统（MemFS）实现
//!
//! 这个模块提供了一个完全在内存中运行的文件系统实现，不依赖任何物理存储设备。
//! 它主要用于临时文件存储、测试目的，或者作为其他文件系统的缓存层。
//!
//! # 特性
//!
//! - 支持文件、目录、符号链接和设备节点
//! - 线程安全的并发访问
//! - 动态inode分配
//! - 完整的VFS接口实现
//! - 支持设备文件（字符设备）
//!
//! # 设计
//!
//! MemFS使用B树来存储目录条目，确保高效的查找和遍历操作。
//! 文件内容存储在可变长度的字节向量中，支持动态调整大小。
//! 所有数据结构都使用读写锁保护，允许多个读取者或单个写入者。
//!
//! # 使用示例
//!
//! ```no_run
//! use crate::fs::vfs::FileSystem;
//! use crate::fs::memfs::MemFs;
//! use alloc::sync::Arc;
//!
//! let memfs = MemFs;
//! let root = memfs.mount(None, None).unwrap();
//! let file = root.create("test.txt", VNodeType::File).unwrap();
//! file.write_at(0, b"Hello, MemFS!").unwrap();
//! ```
//!
//! # 限制
//!
//! - 所有数据都存储在内存中，系统重启后会丢失
//! - 不支持持久化存储
//! - 没有磁盘空间限制（受可用内存限制）

extern crate alloc;
use crate::drivers::Device;
use crate::fs::vfs::{FileSystem, Inode, Metadata, VNodeType, VfsError};
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::any::Any;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::RwLock;

/// 全局原子计数器，用于分配唯一的inode ID
static NEXT_INODE_ID: AtomicUsize = AtomicUsize::new(1);

/// 为指定类型的节点创建元数据
///
/// # 参数
///
/// * `node_type` - 节点类型（文件、目录、符号链接或设备）
/// * `size` - 节点大小（字节）
///
/// # 返回
///
/// 包含默认权限和时间戳的元数据对象
fn create_metadata(node_type: VNodeType, size: u64) -> Metadata {
    let permissions = match node_type {
        VNodeType::Dir => 0o755,
        VNodeType::File => 0o644,
        VNodeType::SymLink => 0o777,
        VNodeType::Device => 0o600,
    };
    Metadata {
        size,
        permissions,
        uid: 0,
        gid: 0,
        ctime: 0,
        mtime: 0,
        blocks: size.div_ceil(512),
        nlinks: 1,
    }
}

/// 内存文件系统节点的内容类型
///
/// 这个枚举定义了内存文件系统中节点可以存储的不同类型的内容。
/// 每种类型都有其特定的数据结构和访问模式。
pub enum MemNodeContent {
    /// 文件节点，包含可变长度的字节数据
    File {
        /// 文件内容，使用读写锁保护以实现并发访问
        data: Arc<RwLock<Vec<u8>>>,
    },
    /// 目录节点，包含子节点映射
    Dir {
        /// 目录条目映射，键为文件名，值为节点引用
        entries: RwLock<BTreeMap<String, Arc<MemVNode>>>,
    },
    /// 符号链接节点，指向另一个路径
    SymLink {
        /// 链接目标路径
        target: String,
    },
    /// 设备节点，引用内核设备
    Device {
        /// 关联的设备对象
        device: Arc<Device>,
    },
}

/// 内存文件系统虚拟节点（VNode）
///
/// 表示内存文件系统中的一个节点（文件、目录、符号链接或设备）。
/// 每个节点都有唯一的ID、类型、元数据和具体内容。
///
/// # 线程安全
///
/// 元数据使用读写锁保护，允许多个读取者或单个写入者。
/// 节点内容的具体保护机制取决于内容类型。
pub struct MemVNode {
    /// 节点的唯一标识符（内部使用）
    #[allow(dead_code)]
    id: usize,
    /// 节点类型（文件、目录、符号链接或设备）
    node_type: VNodeType,
    /// 节点元数据，使用读写锁保护
    metadata: RwLock<Metadata>,
    /// 节点具体内容
    content: MemNodeContent,
}

impl MemVNode {
    /// 创建一个新的内存文件系统节点
    ///
    /// # 参数
    ///
    /// * `node_type` - 节点类型
    /// * `content` - 节点内容
    ///
    /// # 返回
    ///
    /// 新节点的引用计数句柄
    fn new(node_type: VNodeType, content: MemNodeContent) -> Arc<Self> {
        let id = NEXT_INODE_ID.fetch_add(1, Ordering::Relaxed);
        let size = match &content {
            MemNodeContent::File { data } => data.read().len() as u64,
            MemNodeContent::Dir { .. } => 0,
            MemNodeContent::SymLink { target } => target.len() as u64,
            MemNodeContent::Device { .. } => 0,
        };
        Arc::new(Self {
            id,
            node_type,
            metadata: RwLock::new(create_metadata(node_type, size)),
            content,
        })
    }

    /// 更新节点的修改时间（当前实现为空操作）
    ///
    /// 注意：当前实现不实际更新时间戳，保留此方法以备将来扩展。
    fn update_mtime(&self) {}

    /// 更新节点大小并重新计算块数
    ///
    /// # 参数
    ///
    /// * `new_size` - 新的节点大小（字节）
    fn update_size(&self, new_size: u64) {
        let mut meta = self.metadata.write();
        meta.size = new_size;
        meta.blocks = new_size.div_ceil(512);
    }

    /// 将节点从一个目录移动到另一个目录
    ///
    /// 这个操作类似于Unix的`mv`命令，将节点从一个父目录移动到另一个父目录，
    /// 并可能重命名。
    ///
    /// # 参数
    ///
    /// * `source_parent` - 源父目录节点
    /// * `target_parent` - 目标父目录节点
    /// * `old_name` - 源目录中的节点名称
    /// * `new_name` - 目标目录中的新名称
    ///
    /// # 返回
    ///
    /// 成功时返回`Ok(())`，错误时返回相应的`VfsError`
    ///
    /// # 错误
    ///
    /// * `VfsError::NotADirectory` - 源或目标不是目录
    /// * `VfsError::NotFound` - 源节点不存在
    /// * `VfsError::AlreadyExists` - 目标名称已存在
    pub fn move_node(
        source_parent: &Self,
        target_parent: &Self,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), VfsError> {
        let source_entries = match &source_parent.content {
            MemNodeContent::Dir { entries } => entries,
            _ => return Err(VfsError::NotADirectory),
        };

        let target_entries = match &target_parent.content {
            MemNodeContent::Dir { entries } => entries,
            _ => return Err(VfsError::NotADirectory),
        };

        let node_to_move = {
            let source_read = source_entries.read();
            source_read.get(old_name).ok_or(VfsError::NotFound)?.clone()
        };

        {
            let target_read = target_entries.read();
            if target_read.contains_key(new_name) {
                return Err(VfsError::AlreadyExists);
            }
        }

        {
            let mut source_write = source_entries.write();
            source_write.remove(old_name);
        }

        {
            let mut target_write = target_entries.write();
            target_write.insert(new_name.to_string(), node_to_move);
        }

        source_parent.update_mtime();
        target_parent.update_mtime();

        Ok(())
    }
}

impl Inode for MemVNode {
    fn metadata(&self) -> Result<Metadata, VfsError> {
        Ok(self.metadata.read().clone())
    }

    fn set_metadata(&self, metadata: &Metadata) -> Result<(), VfsError> {
        *self.metadata.write() = metadata.clone();
        Ok(())
    }

    fn node_type(&self) -> VNodeType {
        self.node_type
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, VfsError> {
        match &self.content {
            MemNodeContent::File { data } => {
                let data = data.read();
                let data_len = data.len() as u64;
                if offset >= data_len {
                    return Ok(0);
                }
                let bytes_to_read = (data_len - offset).min(buf.len() as u64) as usize;
                let start = offset as usize;
                let end = start + bytes_to_read;
                buf[..bytes_to_read].copy_from_slice(&data[start..end]);
                Ok(bytes_to_read)
            }
            MemNodeContent::Device { device } => {
                if let Some(char_dev) = device.as_char_device() {
                    char_dev.read(buf).map_err(VfsError::DeviceError)
                } else {
                    Err(VfsError::NotImplemented)
                }
            }
            _ => Err(VfsError::NotAFile),
        }
    }

    fn write_at(&self, offset: u64, buf: &[u8]) -> Result<usize, VfsError> {
        match &self.content {
            MemNodeContent::File { data } => {
                let mut data = data.write();
                let start = offset as usize;
                let end = start + buf.len();

                if end > data.len() {
                    data.resize(end, 0);
                }

                data[start..end].copy_from_slice(buf);

                let new_len = data.len() as u64;
                drop(data);
                self.update_size(new_len);

                Ok(buf.len())
            }
            MemNodeContent::Device { device } => {
                if let Some(char_dev) = device.as_char_device() {
                    char_dev.write(buf).map_err(VfsError::DeviceError)
                } else {
                    Err(VfsError::NotImplemented)
                }
            }
            _ => Err(VfsError::NotAFile),
        }
    }

    fn truncate(&self, size: u64) -> Result<(), VfsError> {
        match &self.content {
            MemNodeContent::File { data } => {
                let mut data = data.write();
                data.resize(size as usize, 0);
                let new_len = data.len() as u64;
                drop(data);
                self.update_size(new_len);
                Ok(())
            }
            _ => Err(VfsError::NotImplemented),
        }
    }

    fn sync(&self) -> Result<(), VfsError> {
        Ok(())
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let entries = entries.read();
                entries
                    .get(name)
                    .cloned()
                    .map(|node| node as Arc<dyn Inode>)
                    .ok_or(VfsError::NotFound)
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    fn create(&self, name: &str, typ: VNodeType) -> Result<Arc<dyn Inode>, VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let mut entries = entries.write();
                if entries.contains_key(name) {
                    return Err(VfsError::AlreadyExists);
                }

                let new_node = match typ {
                    VNodeType::File => MemVNode::new(
                        VNodeType::File,
                        MemNodeContent::File {
                            data: Arc::new(RwLock::new(Vec::new())),
                        },
                    ),
                    VNodeType::Dir => MemVNode::new(
                        VNodeType::Dir,
                        MemNodeContent::Dir {
                            entries: RwLock::new(BTreeMap::new()),
                        },
                    ),
                    _ => return Err(VfsError::NotImplemented),
                };

                entries.insert(name.to_string(), new_node.clone());
                self.update_mtime();
                Ok(new_node)
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    fn create_symlink(&self, name: &str, target: &str) -> Result<Arc<dyn Inode>, VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let mut entries = entries.write();
                if entries.contains_key(name) {
                    return Err(VfsError::AlreadyExists);
                }
                let new_node = MemVNode::new(
                    VNodeType::SymLink,
                    MemNodeContent::SymLink {
                        target: target.to_string(),
                    },
                );
                entries.insert(name.to_string(), new_node.clone());
                self.update_mtime();
                Ok(new_node)
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    fn create_device(
        &self,
        name: &str,
        device: Arc<Device>,
    ) -> Result<Arc<dyn Inode>, VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let mut entries = entries.write();
                if entries.contains_key(name) {
                    return Err(VfsError::AlreadyExists);
                }

                let new_node = MemVNode::new(
                    VNodeType::Device,
                    MemNodeContent::Device { device },
                );
                entries.insert(name.to_string(), new_node.clone());
                self.update_mtime();
                Ok(new_node)
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    fn unlink(&self, name: &str) -> Result<(), VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let mut entries = entries.write();
                if let Some(node) = entries.get(name) {
                    if node.node_type() == VNodeType::Dir {
                        if let MemNodeContent::Dir {
                            entries: sub_entries,
                        } = &node.content
                        {
                            if !sub_entries.read().is_empty() {
                                return Err(VfsError::DirectoryNotEmpty);
                            }
                        }
                    }
                    entries.remove(name);
                    self.update_mtime();
                    Ok(())
                } else {
                    Err(VfsError::NotFound)
                }
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    fn rename(&self, old_name: &str, new_name: &str) -> Result<(), VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let mut entries = entries.write();
                if !entries.contains_key(old_name) {
                    return Err(VfsError::NotFound);
                }
                if entries.contains_key(new_name) {
                    return Err(VfsError::AlreadyExists);
                }
                let node = entries.remove(old_name).unwrap();
                entries.insert(new_name.to_string(), node);
                self.update_mtime();
                Ok(())
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    fn list(&self) -> Result<Vec<String>, VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => Ok(entries.read().keys().cloned().collect()),
            _ => Err(VfsError::NotADirectory),
        }
    }

    fn read_symlink(&self) -> Result<String, VfsError> {
        match &self.content {
            MemNodeContent::SymLink { target } => Ok(target.clone()),
            _ => Err(VfsError::NotAFile),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// 内存文件系统实例
///
/// 这个结构体实现了`FileSystem` trait，可以作为文件系统挂载。
/// 它不存储任何状态，所有状态都存储在挂载时创建的根目录节点中。
pub struct MemFs;

impl FileSystem for MemFs {
    /// 挂载内存文件系统
    ///
    /// 创建一个新的内存文件系统实例，返回根目录节点。
    /// 内存文件系统不依赖任何物理设备，因此`device`参数被忽略。
    ///
    /// # 参数
    ///
    /// * `_device` - 物理设备（被忽略）
    /// * `_args` - 挂载参数（被忽略）
    ///
    /// # 返回
    ///
    /// 成功时返回根目录节点的引用，错误时返回`VfsError`
    fn mount(
        &self,
        _device: Option<Arc<Device>>,
        _args: Option<&[&str]>,
    ) -> Result<Arc<dyn Inode>, VfsError> {
        let root_dir = MemVNode::new(
            VNodeType::Dir,
            MemNodeContent::Dir {
                entries: RwLock::new(BTreeMap::new()),
            },
        );
        Ok(root_dir)
    }

    /// 获取文件系统类型标识符
    ///
    /// # 返回
    ///
    /// 文件系统类型字符串："memfs"
    fn fs_type(&self) -> &'static str {
        "memfs"
    }
}

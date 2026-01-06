extern crate alloc;
use crate::drivers::{Device, DEVICE_MANAGER}; // 假设 Device, DeviceError 类型在 crate::drivers 模块中，并引入 DEVICE_MANAGER
use crate::fs::vfs::{File, FileSystem, Metadata, VNode, VNodeType, VfsError};
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicUsize, Ordering}; // 用于生成唯一的 inode ID
use spin::{Mutex, RwLock}; // 用于并发控制

// --- 辅助函数和类型 ---

/// 全局唯一的 inode ID 生成器
static NEXT_INODE_ID: AtomicUsize = AtomicUsize::new(1);

/// 辅助函数：创建一个默认的元数据
fn create_metadata(node_type: VNodeType, size: u64) -> Metadata {
    let permissions = match node_type {
        VNodeType::Dir => 0o755,     // 目录权限：所有者读写执行，组和其他读执行
        VNodeType::File => 0o644,    // 文件权限：所有者读写，组和其他只读
        VNodeType::SymLink => 0o777, // 符号链接权限通常被忽略，但可设置为开放
        VNodeType::Device => 0o600,  // 设备权限：所有者读写，组和其他无权限
    };
    Metadata {
        size,
        permissions,
        uid: 0,                          // 默认用户ID (root)
        gid: 0,                          // 默认组ID (root)
        ctime: 0,                        // 要求所有时间字段为 0
        mtime: 0,                        // 要求所有时间字段为 0
        blocks: size.div_ceil(512),       // 假设块大小为 512 字节
        nlinks: 1,                       // 默认硬链接数为 1
    }
}

/// 表示 MemVNode 的实际内容（文件数据、目录条目或符号链接目标）
pub enum MemNodeContent {
    File {
        data: Arc<RwLock<Vec<u8>>>, // 实际的文件数据，由 Arc<RwLock> 保护
    },
    Dir {
        entries: RwLock<BTreeMap<String, Arc<MemVNode>>>, // 目录条目，映射文件名到子VNode
    },
    SymLink {
        target: String, // 符号链接的目标路径
    },
    Device {
        major: u16,
        minor: u16,
        dev_type: crate::drivers::DeviceType,
    },
}

/// 内存文件系统中的一个节点（文件、目录或符号链接）
pub struct MemVNode {
    #[allow(dead_code)]
    id: usize, // 唯一的标识符，类似 inode 号
    node_type: VNodeType,
    metadata: RwLock<Metadata>, // 元数据，使用 RwLock 保护以允许并发读写
    content: MemNodeContent,
}

impl MemVNode {
    /// 创建一个新的 MemVNode
    fn new(node_type: VNodeType, content: MemNodeContent) -> Arc<Self> {
        let id = NEXT_INODE_ID.fetch_add(1, Ordering::Relaxed);
        let size = match &content {
            MemNodeContent::File { data } => data.read().len() as u64,
            MemNodeContent::Dir { .. } => 0, // 目录通常在 stat 中显示大小为 0
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

    /// 辅助方法：更新节点的修改时间（根据要求，保持为 0）
    fn update_mtime(&self) {
        // self.metadata.write().mtime = get_current_time(); // 如果有实时时钟，我们会在这里更新
    }

    /// 将子节点从一个目录移动到另一个目录。
    /// 这是 MemFs 特有的实现，用于支持跨目录移动。
    pub fn move_node(
        source_parent: &Self,
        target_parent: &Self,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), VfsError> {
        // 获取源目录的条目
        let source_entries = match &source_parent.content {
            MemNodeContent::Dir { entries } => entries,
            _ => return Err(VfsError::NotADirectory),
        };

        // 获取目标目录的条目
        let target_entries = match &target_parent.content {
            MemNodeContent::Dir { entries } => entries,
            _ => return Err(VfsError::NotADirectory),
        };

        // 检查旧名称在源目录中是否存在
        let node_to_move = {
            let source_read = source_entries.read();
            source_read.get(old_name).ok_or(VfsError::NotFound)?.clone()
        };

        // 检查新名称在目标目录中是否已存在
        {
            let target_read = target_entries.read();
            if target_read.contains_key(new_name) {
                return Err(VfsError::AlreadyExists);
            }
        }

        // 从源目录移除
        {
            let mut source_write = source_entries.write();
            source_write.remove(old_name);
        }

        // 添加到目标目录
        {
            let mut target_write = target_entries.write();
            target_write.insert(new_name.to_string(), node_to_move);
        }

        // 更新两个父目录的修改时间
        source_parent.update_mtime();
        target_parent.update_mtime();

        Ok(())
    }
}

/// 实现 VNode trait，定义了文件系统节点的核心行为
impl VNode for MemVNode {
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }

    fn node_type(&self) -> VNodeType {
        self.node_type
    }

    fn metadata(&self) -> Result<Metadata, VfsError> {
        Ok(self.metadata.read().clone())
    }

    fn open(&self) -> Result<Box<dyn File>, VfsError> {
        match &self.content {
            MemNodeContent::File { data } => {
                // 对于文件类型，返回一个 MemFile 实例，它持有对本 VNode 及其数据的引用
                Ok(Box::new(MemFile::new(
                    self.node_type,
                    RwLock::new(self.metadata.read().clone()),
                    (*data).clone(),
                )))
            }
            MemNodeContent::Device {
                major,
                minor,
                dev_type,
            } => {
                // 对于设备文件，尝试通过设备管理器打开
                let device_manager = DEVICE_MANAGER.lock();
                let device = device_manager
                    .get_device_by_major_minor(*major, *minor)
                    .ok_or(VfsError::DeviceNotFound)?;
                if device.device_type() != *dev_type {
                    return Err(VfsError::InvalidArgument); // 类型不匹配
                }
                device.open()?; // 增加设备的打开计数
                Ok(Box::new(MemDeviceFile::new(device.clone())))
            }
            _ => Err(VfsError::NotAFile), // 只有文件和设备可以被打开进行读写
        }
    }

    fn lookup(&self, name: &str) -> Result<Arc<dyn VNode>, VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let entries_read = entries.read();
                // 优化：在`map`中将`MemVNode`转换为`dyn VNode`，避免提前clone
                entries_read
                    .get(name)
                    .map(|node| node.clone() as Arc<dyn VNode>)
                    .ok_or(VfsError::NotFound)
            }
            _ => Err(VfsError::NotADirectory), // 只有目录可以进行查找操作
        }
    }

    fn create(&self, name: &str, typ: VNodeType) -> Result<Arc<dyn VNode>, VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let mut entries_write = entries.write();
                if entries_write.contains_key(name) {
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
                    VNodeType::SymLink => {
                        return Err(VfsError::NotImplemented);
                    } // 符号链接有专门的create_symlink
                    VNodeType::Device => {
                        return Err(VfsError::NotImplemented);
                    } // 设备节点有专门的create_device
                };
                entries_write.insert(name.to_string(), new_node.clone());
                self.update_mtime(); // 父目录的修改时间变化
                Ok(new_node)
            }
            _ => Err(VfsError::NotADirectory), // 只有目录可以创建子节点
        }
    }

    fn create_device(
        &self,
        name: &str,
        major: u16,
        minor: u16,
        device_type: crate::drivers::DeviceType,
    ) -> Result<Arc<dyn VNode>, VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let mut entries_write = entries.write();
                if entries_write.contains_key(name) {
                    return Err(VfsError::AlreadyExists);
                }

                // 在创建设备节点前，先确认设备管理器中存在该设备
                let device_manager = DEVICE_MANAGER.lock();
                let _ = device_manager
                    .get_device_by_major_minor(major, minor)
                    .ok_or(VfsError::DeviceNotFound)?;
                drop(device_manager); // 释放锁

                let new_node = MemVNode::new(
                    VNodeType::Device,
                    MemNodeContent::Device {
                        major,
                        minor,
                        dev_type: device_type,
                    },
                );
                entries_write.insert(name.to_string(), new_node.clone());
                self.update_mtime();
                Ok(new_node)
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    fn create_symlink(&self, name: &str, target_path: &str) -> Result<Arc<dyn VNode>, VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let mut entries_write = entries.write();
                if entries_write.contains_key(name) {
                    return Err(VfsError::AlreadyExists);
                }
                let new_node = MemVNode::new(
                    VNodeType::SymLink,
                    MemNodeContent::SymLink {
                        target: target_path.to_string(),
                    },
                );
                entries_write.insert(name.to_string(), new_node.clone());
                self.update_mtime();
                Ok(new_node)
            }
            _ => Err(VfsError::NotADirectory),
        }
    }

    fn remove(&self, name: &str) -> Result<(), VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let mut entries_write = entries.write();
                if let Some(node_to_remove) = entries_write.get(name) {
                    if node_to_remove.node_type() == VNodeType::Dir {
                        // 检查目录是否为空
                        if let MemNodeContent::Dir {
                            entries: sub_entries,
                        } = &node_to_remove.content
                        {
                            if !sub_entries.read().is_empty() {
                                return Err(VfsError::DirectoryNotEmpty);
                            }
                        }
                    }
                    entries_write.remove(name);
                    self.update_mtime(); // 父目录的修改时间变化
                    Ok(())
                } else {
                    Err(VfsError::NotFound)
                }
            }
            _ => Err(VfsError::NotADirectory), // 只有目录可以删除子节点
        }
    }

    fn rename(&self, old_name: &str, new_name: &str) -> Result<(), VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => {
                let mut entries_write = entries.write();

                // 检查旧名称是否存在
                if !entries_write.contains_key(old_name) {
                    return Err(VfsError::NotFound);
                }

                // 检查新名称是否已存在
                if entries_write.contains_key(new_name) {
                    return Err(VfsError::AlreadyExists);
                }

                // 获取要重命名的节点
                let node = entries_write
                    .get(old_name)
                    .ok_or(VfsError::NotFound)?
                    .clone();

                // 移除旧名称，插入新名称
                entries_write.remove(old_name);
                entries_write.insert(new_name.to_string(), node);

                self.update_mtime(); // 父目录的修改时间变化
                Ok(())
            }
            _ => Err(VfsError::NotADirectory), // 只有目录可以重命名子节点
        }
    }

    fn read_symlink(&self) -> Result<String, VfsError> {
        match &self.content {
            MemNodeContent::SymLink { target } => Ok(target.clone()),
            _ => Err(VfsError::NotAFile), // 根据 VFS 定义，非符号链接返回 NotAFile 错误
        }
    }

    fn read_dir(&self) -> Result<Vec<String>, VfsError> {
        match &self.content {
            MemNodeContent::Dir { entries } => Ok(entries.read().keys().cloned().collect()),
            _ => Err(VfsError::NotADirectory),
        }
    }
}

/// 表示内存文件系统中的一个打开文件句柄
pub struct MemFile {
    #[allow(dead_code)]
    node_type: VNodeType,
    metadata: RwLock<Metadata>,     // 文件的元数据，用于 stat
    data_ref: Arc<RwLock<Vec<u8>>>, // 对实际文件数据的引用
    cursor: Mutex<u64>,             // 当前读写位置
}

impl MemFile {
    fn new(
        node_type: VNodeType,
        metadata: RwLock<Metadata>,
        data_ref: Arc<RwLock<Vec<u8>>>,
    ) -> Self {
        Self {
            node_type,
            metadata,
            data_ref,
            cursor: Mutex::new(0),
        }
    }

    /// 辅助方法：更新文件的元数据（大小和修改时间）
    fn update_metadata(&self, new_size: u64) {
        let mut metadata_write = self.metadata.write();
        metadata_write.size = new_size;
        metadata_write.blocks = new_size.div_ceil(512);
        // metadata_write.mtime = get_current_time(); // 根据要求，mtime 保持为 0
    }
}

/// 实现 File trait，定义了文件操作行为
impl File for MemFile {
    fn read(&self, buf: &mut [u8]) -> Result<usize, VfsError> {
        let mut cursor = self.cursor.lock();
        let data = self.data_ref.read();
        let data_len = data.len() as u64;

        if *cursor >= data_len {
            return Ok(0); // 文件末尾
        }

        let bytes_to_read = (data_len - *cursor).min(buf.len() as u64) as usize;
        let start_idx = *cursor as usize;
        let end_idx = start_idx + bytes_to_read;

        buf[..bytes_to_read].copy_from_slice(&data[start_idx..end_idx]);
        *cursor += bytes_to_read as u64;
        Ok(bytes_to_read)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, VfsError> {
        let mut cursor = self.cursor.lock();
        let mut data = self.data_ref.write();
        let _data_len = data.len() as u64;

        let start_idx = *cursor as usize;
        let bytes_to_write = buf.len();

        // 如果写入位置超出当前文件大小，则扩展文件并用零填充
        if start_idx + bytes_to_write > data.len() {
            data.resize(start_idx + bytes_to_write, 0);
        }

        data[start_idx..start_idx + bytes_to_write].copy_from_slice(buf);

        *cursor += bytes_to_write as u64;
        self.update_metadata(data.len() as u64); // 更新文件大小和修改时间
        Ok(bytes_to_write)
    }

    fn seek(&self, pos: u64) -> Result<u64, VfsError> {
        let mut cursor = self.cursor.lock();
        // 允许 seek 到文件末尾之外，POSIX 文件系统通常支持
        *cursor = pos;
        Ok(*cursor)
    }

    fn stat(&self) -> Result<Metadata, VfsError> {
        // 确保元数据中的 size 始终与实际数据长度保持一致
        let mut metadata_write = self.metadata.write();
        let current_data_len = self.data_ref.read().len() as u64;
        if metadata_write.size != current_data_len {
            metadata_write.size = current_data_len;
            metadata_write.blocks = current_data_len.div_ceil(512);
            // metadata_write.mtime = get_current_time(); // 如果有实时时钟，这里会更新
        }
        Ok(metadata_write.clone())
    }

    fn truncate(&mut self, size: u64) -> Result<(), VfsError> {
        let mut data = self.data_ref.write();
        if size < data.len() as u64 {
            data.truncate(size as usize);
        } else if size > data.len() as u64 {
            // 如果新大小更大，则用零扩展文件
            data.resize(size as usize, 0);
        }
        self.update_metadata(data.len() as u64); // 更新文件大小和修改时间
        Ok(())
    }
}

// 新增：表示内存文件系统中的一个打开的设备文件句柄
pub struct MemDeviceFile {
    device: Device, // 对实际设备的引用
}

impl MemDeviceFile {
    fn new(device: Device) -> Self {
        Self { device }
    }
}

impl File for MemDeviceFile {
    fn read(&self, buf: &mut [u8]) -> Result<usize, VfsError> {
        match self.device.as_char_device() {
            Some(char_dev) => char_dev.read(buf).map_err(VfsError::DeviceError),
            None => Err(VfsError::NotImplemented), // 不支持从块设备直接读文件接口
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, VfsError> {
        match self.device.as_char_device() {
            Some(char_dev) => char_dev.write(buf).map_err(VfsError::DeviceError),
            None => Err(VfsError::NotImplemented), // 不支持写入块设备
        }
    }

    fn seek(&self, _pos: u64) -> Result<u64, VfsError> {
        // 设备文件通常不支持 seek
        Err(VfsError::NotSupported)
    }

    fn stat(&self) -> Result<Metadata, VfsError> {
        // 设备文件的元数据可能需要从设备驱动获取，这里返回一个默认值
        Ok(create_metadata(VNodeType::Device, 0))
    }

    fn truncate(&mut self, _size: u64) -> Result<(), VfsError> {
        // 设备文件不支持 truncate
        Err(VfsError::NotSupported)
    }
}

/// 内存文件系统驱动结构体
pub struct MemFs;

/// 实现 FileSystem trait，定义了文件系统的挂载行为
impl FileSystem for MemFs {
    fn mount(
        &self,
        _device: Option<&Device>, // MemFs 是内存文件系统，不依赖于物理设备
        _args: Option<&[&str]>,
    ) -> Result<Arc<dyn VNode>, VfsError> {
        // 为这个 MemFs 实例创建根目录
        let root_dir = MemVNode::new(
            VNodeType::Dir,
            MemNodeContent::Dir {
                entries: RwLock::new(BTreeMap::new()),
            },
        );
        Ok(root_dir)
    }

    fn fs_type(&self) -> &'static str {
        "memfs"
    }
}

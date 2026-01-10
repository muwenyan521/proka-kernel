//! 虚拟文件系统（VFS）层
//!
//! 这个模块提供了文件系统的抽象层，允许不同的文件系统实现（如MemFS、KernFS等）
//! 通过统一的接口进行交互。VFS层处理路径解析、挂载点管理和文件操作调度。
//!
//! # 核心概念
//!
//! - **Inode**: 文件系统节点的抽象，表示文件、目录、符号链接或设备
//! - **FileSystem**: 文件系统实现的trait，提供挂载功能
//! - **Vfs**: 虚拟文件系统管理器，处理挂载点和路径解析
//! - **File**: 文件句柄，提供读写操作接口
//!
//! # 支持的文件系统
//!
//! - `memfs`: 内存文件系统（见`memfs`模块）
//! - `kernfs`: 内核文件系统（见`kernfs`模块）
//!
//! # 设计原则
//!
//! 1. **统一接口**: 所有文件系统通过相同的trait进行交互
//! 2. **路径透明**: 应用程序无需关心底层文件系统类型
//! 3. **挂载管理**: 支持多个文件系统挂载到不同路径
//! 4. **符号链接**: 支持符号链接解析（有深度限制）
//! 5. **线程安全**: 所有操作都是线程安全的
//!
//! # 使用示例
//!
//! ```no_run
//! use crate::fs::vfs::VFS;
//! use alloc::sync::Arc;
//!
//! // 打开文件
//! let file = VFS.open("/kernel/test.txt")?;
//! let mut buffer = [0u8; 1024];
//! let bytes_read = file.read(&mut buffer)?;
//!
//! // 创建目录
//! VFS.create_dir("/kernel/new_dir")?;
//!
//! // 列出目录内容
//! let entries = VFS.read_dir("/kernel")?;
//! ```
//!
//! # 限制
//!
//! - 符号链接解析深度限制为8层（防止循环链接）
//! - 不支持硬链接
//! - 文件权限检查是基本的
//! - 时间戳支持有限

use crate::drivers::{Device, DeviceError, DEVICE_MANAGER};
extern crate alloc;
use super::kernfs::KernFs;
use super::memfs::MemFs;
use alloc::format;
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::any::Any;
use lazy_static::lazy_static;
use spin::{Mutex, RwLock};

lazy_static! {
    /// 全局虚拟文件系统实例
    ///
    /// 这是整个内核使用的单一VFS实例。它预先注册了`memfs`和`kernfs`文件系统，
    /// 并将`/kernel`路径挂载到内核文件系统。
    pub static ref VFS: Vfs = {
        let fs = Vfs::new();
        fs
    };
}

/// 虚拟文件系统错误类型
///
/// 表示VFS操作中可能发生的各种错误。
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum VfsError {
    /// 文件或目录不存在
    NotFound,
    /// 文件或目录已存在
    AlreadyExists,
    /// 路径不是目录
    NotADirectory,
    /// 路径不是文件
    NotAFile,
    /// 权限不足
    PermissionDenied,
    /// 设备错误
    DeviceError(DeviceError),
    /// 无效的参数
    InvalidArgument,
    /// IO 错误
    IoError,
    /// 符号链接深度过深（超过最大深度限制）
    MaxSymlinkDepth,
    /// 文件系统类型不支持
    FsTypeNotSupported,
    /// 路径为空
    EmptyPath,
    /// 功能未实现
    NotImplemented,
    /// 目录非空（尝试删除包含文件的目录）
    DirectoryNotEmpty,
}

impl From<DeviceError> for VfsError {
    /// 将设备错误转换为VFS错误
    fn from(e: DeviceError) -> Self {
        VfsError::DeviceError(e)
    }
}

/// 虚拟节点类型
///
/// 表示文件系统中节点的类型。这决定了节点可以执行的操作类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VNodeType {
    /// 普通文件，包含数据
    File,
    /// 目录，包含其他节点
    Dir,
    /// 符号链接，指向另一个路径
    SymLink,
    /// 设备文件，关联到内核设备
    Device,
}

/// 文件或目录的元数据
///
/// 包含文件的统计信息，类似于Unix的`stat`结构。
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Metadata {
    /// 文件大小（字节）
    pub size: u64,
    /// UNIX权限位，如0o755（所有者/组/其他 读/写/执行）
    pub permissions: u32,
    /// 用户ID（文件所有者）
    pub uid: u32,
    /// 组ID（文件所属组）
    pub gid: u32,
    /// 创建时间（秒，从某个纪元开始）
    pub ctime: u64,
    /// 最后修改时间（秒，从某个纪元开始）
    pub mtime: u64,
    /// 占用的块数（每个块512字节）
    pub blocks: u64,
    /// 硬链接数量
    pub nlinks: u64,
}

/// 文件系统trait
///
/// 所有文件系统实现必须实现这个trait。它定义了文件系统的基本操作：
/// 挂载和获取文件系统类型。
pub trait FileSystem: Send + Sync {
    /// 挂载文件系统
    ///
    /// # 参数
    ///
    /// * `device` - 可选的物理设备（对于基于设备的文件系统）
    /// * `args` - 可选的挂载参数
    ///
    /// # 返回
    ///
    /// 成功时返回根目录节点的引用，错误时返回`VfsError`
    fn mount(
        &self,
        device: Option<Arc<Device>>,
        args: Option<&[&str]>,
    ) -> Result<Arc<dyn Inode>, VfsError>;

    /// 获取文件系统类型标识符
    ///
    /// # 返回
    ///
    /// 文件系统类型字符串（如"memfs"、"kernfs"等）
    fn fs_type(&self) -> &'static str;
}

/// 索引节点（Inode）trait
///
/// 表示文件系统中的一个节点（文件、目录、符号链接或设备）。
/// 这个trait定义了所有节点类型必须支持的操作。
pub trait Inode: Send + Sync {
    /// 获取节点元数据
    fn metadata(&self) -> Result<Metadata, VfsError>;

    /// 设置节点元数据
    fn set_metadata(&self, metadata: &Metadata) -> Result<(), VfsError>;

    /// 获取节点类型
    fn node_type(&self) -> VNodeType;

    /// 从指定偏移量读取数据
    ///
    /// # 参数
    ///
    /// * `offset` - 读取起始偏移量（字节）
    /// * `buf` - 存储读取数据的缓冲区
    ///
    /// # 返回
    ///
    /// 成功时返回实际读取的字节数，错误时返回`VfsError`
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, VfsError>;

    /// 向指定偏移量写入数据
    ///
    /// # 参数
    ///
    /// * `offset` - 写入起始偏移量（字节）
    /// * `buf` - 要写入的数据
    ///
    /// # 返回
    ///
    /// 成功时返回实际写入的字节数，错误时返回`VfsError`
    fn write_at(&self, offset: u64, buf: &[u8]) -> Result<usize, VfsError>;

    /// 截断文件到指定大小
    ///
    /// # 参数
    ///
    /// * `size` - 新的文件大小（字节）
    fn truncate(&self, size: u64) -> Result<(), VfsError>;

    /// 同步节点数据到存储设备（如果适用）
    fn sync(&self) -> Result<(), VfsError>;

    /// 在目录中查找子节点
    ///
    /// # 参数
    ///
    /// * `name` - 子节点名称
    ///
    /// # 返回
    ///
    /// 成功时返回子节点的引用，错误时返回`VfsError`
    ///
    /// # 注意
    ///
    /// 只有目录节点支持此操作
    fn lookup(&self, name: &str) -> Result<Arc<dyn Inode>, VfsError>;

    /// 在目录中创建新节点
    ///
    /// # 参数
    ///
    /// * `name` - 新节点名称
    /// * `typ` - 节点类型
    ///
    /// # 返回
    ///
    /// 成功时返回新节点的引用，错误时返回`VfsError`
    ///
    /// # 注意
    ///
    /// 只有目录节点支持此操作
    fn create(&self, name: &str, typ: VNodeType) -> Result<Arc<dyn Inode>, VfsError>;

    /// 创建符号链接（默认实现返回未实现错误）
    fn create_symlink(&self, name: &str, target: &str) -> Result<Arc<dyn Inode>, VfsError> {
        let _ = (name, target);
        Err(VfsError::NotImplemented)
    }

    /// 创建设备节点（默认实现返回未实现错误）
    fn create_device(
        &self,
        name: &str,
        device: Arc<Device>,
    ) -> Result<Arc<dyn Inode>, VfsError> {
        let _ = (name, device);
        Err(VfsError::NotImplemented)
    }

    /// 删除目录中的节点
    ///
    /// # 参数
    ///
    /// * `name` - 要删除的节点名称
    ///
    /// # 注意
    ///
    /// 只有目录节点支持此操作
    fn unlink(&self, name: &str) -> Result<(), VfsError>;

    /// 重命名节点（默认实现返回未实现错误）
    fn rename(&self, old_name: &str, new_name: &str) -> Result<(), VfsError> {
        let _ = (old_name, new_name);
        Err(VfsError::NotImplemented)
    }

    /// 列出目录内容（默认实现返回空列表）
    fn list(&self) -> Result<Vec<String>, VfsError> {
        Ok(Vec::new())
    }

    /// 读取符号链接目标（默认实现返回不是文件的错误）
    fn read_symlink(&self) -> Result<String, VfsError> {
        Err(VfsError::NotAFile)
    }

    /// 将节点转换为`Any` trait对象，用于向下转换
    fn as_any(&self) -> &dyn Any;
}

/// 文件句柄
///
/// 表示一个打开的文件，维护当前读写位置。
/// 提供比原始Inode更友好的文件操作接口。
pub struct File {
    /// 底层节点引用
    inode: Arc<dyn Inode>,
    /// 当前文件偏移量（字节），使用互斥锁保护以实现线程安全
    offset: Mutex<u64>,
}

impl File {
    /// 创建新的文件句柄
    ///
    /// # 参数
    ///
    /// * `inode` - 底层节点引用
    pub fn new(inode: Arc<dyn Inode>) -> Self {
        Self {
            inode,
            offset: Mutex::new(0),
        }
    }

    /// 从当前偏移量读取数据
    ///
    /// 读取数据并自动更新文件偏移量。
    ///
    /// # 参数
    ///
    /// * `buf` - 存储读取数据的缓冲区
    ///
    /// # 返回
    ///
    /// 成功时返回实际读取的字节数，错误时返回`VfsError`
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, VfsError> {
        let mut offset = self.offset.lock();
        let len = self.inode.read_at(*offset, buf)?;
        *offset += len as u64;
        Ok(len)
    }

    /// 向当前偏移量写入数据
    ///
    /// 写入数据并自动更新文件偏移量。
    ///
    /// # 参数
    ///
    /// * `buf` - 要写入的数据
    ///
    /// # 返回
    ///
    /// 成功时返回实际写入的字节数，错误时返回`VfsError`
    pub fn write(&self, buf: &[u8]) -> Result<usize, VfsError> {
        let mut offset = self.offset.lock();
        let len = self.inode.write_at(*offset, buf)?;
        *offset += len as u64;
        Ok(len)
    }

    /// 移动文件偏移量
    ///
    /// # 参数
    ///
    /// * `pos` - 新的文件偏移量（字节）
    ///
    /// # 返回
    ///
    /// 成功时返回新的文件偏移量，错误时返回`VfsError`
    pub fn seek(&self, pos: u64) -> Result<u64, VfsError> {
        let mut offset = self.offset.lock();
        *offset = pos;
        Ok(*offset)
    }

    /// 获取文件元数据
    pub fn metadata(&self) -> Result<Metadata, VfsError> {
        self.inode.metadata()
    }

    /// 截断文件到指定大小
    ///
    /// # 参数
    ///
    /// * `size` - 新的文件大小（字节）
    pub fn truncate(&self, size: u64) -> Result<(), VfsError> {
        self.inode.truncate(size)
    }

    /// 读取整个文件内容
    ///
    /// 从文件开头读取所有数据，忽略当前偏移量。
    ///
    /// # 返回
    ///
    /// 成功时返回文件内容的字节向量，错误时返回`VfsError`
    pub fn read_all(&self) -> Result<Vec<u8>, VfsError> {
        let metadata = self.metadata()?;
        let mut buf = alloc::vec![0; metadata.size as usize];
        self.inode.read_at(0, &mut buf)?;
        Ok(buf)
    }

    /// 写入整个文件内容
    ///
    /// 从文件开头写入所有数据，覆盖现有内容。
    ///
    /// # 参数
    ///
    /// * `data` - 要写入的数据
    pub fn write_all(&self, data: &[u8]) -> Result<(), VfsError> {
        self.inode.write_at(0, data)?;
        Ok(())
    }
}

/// 挂载点信息（内部使用）
struct MountPoint {
    /// 挂载点路径（规范化后，不包含首尾斜杠）
    path: String,
    /// 路径长度（用于快速比较）
    mount_point_len: usize,
    /// 挂载的文件系统根节点
    root: Arc<dyn Inode>,
}

/// 虚拟文件系统管理器
///
/// 管理文件系统注册、挂载点和路径解析。
/// 提供统一的文件操作接口，隐藏底层文件系统细节。
pub struct Vfs {
    /// 根文件系统节点（可选）
    root: RwLock<Option<Arc<dyn Inode>>>,
    /// 挂载点列表，按路径长度排序（最长的优先）
    mounts: Mutex<Vec<MountPoint>>,
    /// 文件系统类型注册表
    fs_registry: RwLock<BTreeMap<&'static str, Arc<dyn FileSystem>>>,
}

impl Vfs {
    pub fn new() -> Self {
        let mut registry: BTreeMap<&'static str, Arc<dyn FileSystem>> = BTreeMap::new();
        let kernfs = Arc::new(KernFs::new());
        registry.insert("kernfs", kernfs.clone());
        registry.insert("memfs", Arc::new(MemFs));

        let mounts = alloc::vec![MountPoint {
            path: "kernel".to_string(),
            mount_point_len: 6,
            root: kernfs.root(),
        }];

        Self {
            root: RwLock::new(None),
            mounts: Mutex::new(mounts),
            fs_registry: RwLock::new(registry),
        }
    }

    pub fn init_root(&self, root: Arc<dyn Inode>) {
        *self.root.write() = Some(root);
    }

    pub fn register_fs(&self, fs: Arc<dyn FileSystem>) {
        self.fs_registry.write().insert(fs.fs_type(), fs);
    }

    pub fn mount(
        &self,
        device_str: Option<&str>,
        mount_point: &str,
        fs_type: &str,
        args: Option<&[&str]>,
    ) -> Result<(), VfsError> {
        let fs = self
            .fs_registry
            .read()
            .get(fs_type)
            .cloned()
            .ok_or(VfsError::FsTypeNotSupported)?;

        let device_manager = DEVICE_MANAGER.read();
        let device = if let Some(dev) = device_str {
            device_manager.get_device(dev)
        } else {
            None
        };

        let root_inode = fs.mount(device, args)?;

        let parent_path = if mount_point == "/" {
            None
        } else {
            mount_point.rsplit_once('/').map(|(p, _)| p)
        };
        if let Some(parent_path_str) = parent_path {
            let parent_inode = self.lookup(parent_path_str)?;
            if parent_inode.node_type() != VNodeType::Dir {
                return Err(VfsError::NotADirectory);
            }
        }

        let normalized_mount_point = mount_point.trim_matches('/').to_string();
        if normalized_mount_point.is_empty() {
            return Err(VfsError::InvalidArgument);
        }

        self.mounts.lock().push(MountPoint {
            path: normalized_mount_point.clone(),
            mount_point_len: normalized_mount_point.len(),
            root: root_inode,
        });

        Ok(())
    }

    pub fn lookup(&self, path: &str) -> Result<Arc<dyn Inode>, VfsError> {
        self.path_to_inode(path, 0)
    }

    pub fn open(&self, path: &str) -> Result<Arc<File>, VfsError> {
        let inode = self.lookup(path)?;
        if inode.node_type() != VNodeType::File && inode.node_type() != VNodeType::Device {
            return Err(VfsError::NotAFile);
        }
        Ok(Arc::new(File::new(inode)))
    }

    pub fn create_file(&self, path: &str) -> Result<Arc<dyn Inode>, VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_inode = self.lookup(parent_path)?;
        if parent_inode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        parent_inode.create(name, VNodeType::File)
    }

    pub fn create_dir(&self, path: &str) -> Result<Arc<dyn Inode>, VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_inode = self.lookup(parent_path)?;
        if parent_inode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        parent_inode.create(name, VNodeType::Dir)
    }

    pub fn create_symlink(
        &self,
        target_path: &str,
        link_path: &str,
    ) -> Result<Arc<dyn Inode>, VfsError> {
        let (parent_path, name) = self.split_path(link_path)?;
        let parent_inode = self.lookup(parent_path)?;
        if parent_inode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        parent_inode.create_symlink(name, target_path)
    }

    pub fn create_device_node(
        &self,
        path: &str,
        major: u16,
        minor: u16,
        device_type: crate::drivers::DeviceType,
    ) -> Result<Arc<dyn Inode>, VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_inode = self.lookup(parent_path)?;
        if parent_inode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }

        let device_manager = DEVICE_MANAGER.read();
        let device = device_manager
            .get_device_by_major_minor(major, minor)
            .ok_or(VfsError::DeviceError(crate::drivers::DeviceError::NoSuchDevice))?;

        if device.device_type() != device_type {
            return Err(VfsError::DeviceError(crate::drivers::DeviceError::InvalidParam));
        }

        parent_inode.create_device(name, device)
    }

    pub fn remove(&self, path: &str) -> Result<(), VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_inode = self.lookup(parent_path)?;
        if parent_inode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        parent_inode.unlink(name)
    }

    pub fn read_dir(&self, path: &str) -> Result<Vec<String>, VfsError> {
        let inode = self.lookup(path)?;
        if inode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        inode.list()
    }

    pub fn rename(&self, old_path: &str, new_path: &str) -> Result<(), VfsError> {
        let (old_parent_path, old_name) = self.split_path(old_path)?;
        let old_parent = self.lookup(old_parent_path)?;

        let (new_parent_path, new_name) = self.split_path(new_path)?;
        let _new_parent = self.lookup(new_parent_path)?;

        if old_parent_path == new_parent_path {
            old_parent.rename(old_name, new_name)
        } else {
            Err(VfsError::NotImplemented)
        }
    }

    const MAX_SYMLINK_DEPTH: u32 = 8;

    fn path_to_inode(&self, path: &str, depth: u32) -> Result<Arc<dyn Inode>, VfsError> {
        if depth > Self::MAX_SYMLINK_DEPTH {
            return Err(VfsError::MaxSymlinkDepth);
        }

        let normalized_path = self.normalize_path(path);
        let path = normalized_path.trim_matches('/');

        if path.is_empty() {
            return self.root.read().as_ref().cloned().ok_or(VfsError::NotFound);
        }

        let mounts = self.mounts.lock();
        if let Some(mount) = mounts
            .iter()
            .filter(|m| path.starts_with(&m.path))
            .max_by_key(|m| m.mount_point_len)
        {
            let subpath = path[mount.mount_point_len..].trim_matches('/');
            if subpath.is_empty() {
                return self.resolve_symlink_if_needed(mount.root.clone(), depth);
            }
            let mut current = mount.root.clone();
            for component in subpath.split('/') {
                if component.is_empty() || component == "." {
                    continue;
                }
                current = self.resolve_symlink_if_needed(current.lookup(component)?, depth)?;
            }
            return Ok(current);
        }

        let mut current = self
            .root
            .read()
            .as_ref()
            .cloned()
            .ok_or(VfsError::NotFound)?;
        for component in path.split('/') {
            if component.is_empty() || component == "." {
                continue;
            }
            current = self.resolve_symlink_if_needed(current.lookup(component)?, depth)?;
        }
        Ok(current)
    }

    fn resolve_symlink_if_needed(
        &self,
        node: Arc<dyn Inode>,
        depth: u32,
    ) -> Result<Arc<dyn Inode>, VfsError> {
        if node.node_type() == VNodeType::SymLink {
            let target_path = node.read_symlink()?;
            self.path_to_inode(&target_path, depth + 1)
        } else {
            Ok(node)
        }
    }

    fn split_path<'a>(&self, path: &'a str) -> Result<(&'a str, &'a str), VfsError> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Err(VfsError::EmptyPath);
        }
        if let Some((parent, name)) = path.rsplit_once('/') {
            Ok(((if parent.is_empty() { "/" } else { parent }), name))
        } else {
            Ok(("/", path))
        }
    }

    pub fn walk(
        &self,
        start_path: &str,
    ) -> Result<Vec<(String, Vec<String>, Vec<String>)>, VfsError> {
        let mut results = Vec::new();
        let mut stack: Vec<String> = Vec::new();
        let normalized_start_path = self.normalize_path(start_path);
        stack.push(normalized_start_path);
        while let Some(current_dir_path) = stack.pop() {
            let current_inode = self.lookup(&current_dir_path)?;
            if current_inode.node_type() != VNodeType::Dir {
                continue;
            }
            let entries = current_inode.list()?;
            let mut dirnames: Vec<String> = Vec::new();
            let mut filenames: Vec<String> = Vec::new();
            let mut subdirs_to_visit: Vec<String> = Vec::new();

            for entry_name in entries {
                let entry_path = if current_dir_path == "/" {
                    format!("/{}", entry_name)
                } else {
                    format!("{}/{}", current_dir_path, entry_name)
                };

                let entry_inode = match self.lookup(&entry_path) {
                    Ok(node) => node,
                    Err(_) => continue,
                };

                match entry_inode.node_type() {
                    VNodeType::Dir => {
                        dirnames.push(entry_name.clone());
                        subdirs_to_visit.push(entry_path);
                    }
                    _ => {
                        filenames.push(entry_name.clone());
                    }
                }
            }
            results.push((current_dir_path.clone(), dirnames, filenames));
            subdirs_to_visit.sort_unstable_by(|a, b| b.cmp(a));
            for subdir_path in subdirs_to_visit {
                stack.push(subdir_path);
            }
        }
        Ok(results)
    }

    fn normalize_path(&self, path: &str) -> String {
        let parts: Vec<&str> = path
            .split('/')
            .filter(|&s| !s.is_empty() && s != ".")
            .collect();
        let mut cleaned_parts = Vec::new();
        for part in parts.iter() {
            if *part == ".." {
                if let Some(last) = cleaned_parts.last_mut() {
                    if last != &"" {
                        cleaned_parts.pop();
                    }
                }
            } else {
                cleaned_parts.push(*part);
            }
        }
        if cleaned_parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", cleaned_parts.join("/"))
        }
    }
}

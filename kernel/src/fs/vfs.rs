use crate::drivers::{DEVICE_MANAGER, Device, DeviceError};
extern crate alloc;
use super::memfs::MemFs; // 假设 MemFs 在 drivers/memfs.rs 或类似位置
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use lazy_static::lazy_static;
use spin::{Mutex, RwLock};

lazy_static! {
    /// 全局唯一的虚拟文件系统实例
    pub static ref VFS: Mutex<Vfs> = Mutex::new(Vfs::new());
}

/// VFS操作可能返回的错误类型
#[derive(Debug)]
pub enum VfsError {
    NotFound,
    AlreadyExists,
    NotADirectory,
    NotAFile,
    PermissionDenied,
    DeviceError(DeviceError),
    InvalidArgument,
    IoError,
    /// 符号链接解析深度超过限制
    MaxSymlinkDepth,
    FsTypeNotSupported,
    DeviceNotFound,
    EmptyPath,
    NotImplemented,
}

impl From<DeviceError> for VfsError {
    fn from(e: DeviceError) -> Self {
        VfsError::DeviceError(e)
    }
}

// --- 基础类型定义 ---

/// 虚拟文件系统节点的类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VNodeType {
    File,
    Dir,
    SymLink,
    Device, // Block or Char device
}

/// 文件或目录的元数据
#[derive(Debug, Clone)]
pub struct Metadata {
    pub size: u64,
    pub permissions: u32, // UNIX权限位，如0o755
    pub uid: u32,
    pub gid: u32,
    pub ctime: u64, // 创建时间 (秒或毫秒，取决于系统)
    pub mtime: u64, // 最后修改时间 (秒或毫秒)
    // pub atime: u64, // 最后访问时间 (如果需要的话)
    pub blocks: u64, // 占用的块数
    pub nlinks: u64, // 硬链接数量
}

/// 文件操作接口
pub trait File: Send + Sync {
    /// 从文件当前位置读取数据到缓冲区。返回读取的字节数。
    fn read(&self, buf: &mut [u8]) -> Result<usize, VfsError>;
    /// 将缓冲区数据写入文件当前位置。返回写入的字节数。
    fn write(&mut self, buf: &[u8]) -> Result<usize, VfsError>;
    /// 移动文件指针到指定位置。返回新的文件指针位置。
    fn seek(&self, pos: u64) -> Result<u64, VfsError>;
    /// 获取文件元数据。
    fn stat(&self) -> Result<Metadata, VfsError>;
    /// 获取文件大小。
    fn len(&self) -> Result<u64, VfsError> {
        self.stat().map(|m| m.size)
    }
    /// 截断文件到指定大小。
    fn truncate(&mut self, size: u64) -> Result<(), VfsError> {
        let _ = size; // 默认实现什么也不做
        Err(VfsError::NotImplemented)
    }
    // 更多文件操作如 flush, ioctl 等可在此添加
}

/// 文件系统实现接口
pub trait FileSystem: Send + Sync {
    /// 挂载文件系统。返回文件系统的根VNode。
    /// device: 实际设备（如块设备），如果文件系统是内存型的则为None。
    /// args: 挂载参数。
    fn mount(
        &self,
        device: Option<&Device>,
        args: Option<&[&str]>,
    ) -> Result<Arc<dyn VNode>, VfsError>;
    /// 返回文件系统类型的字符串标识符 (如 "memfs", "ext2")。
    fn fs_type(&self) -> &'static str;
}

/// 虚拟文件系统节点接口 (文件、目录、符号链接、设备)
pub trait VNode: Send + Sync {
    /// 返回节点的类型。
    fn node_type(&self) -> VNodeType;
    /// 获取节点的元数据。
    fn metadata(&self) -> Result<Metadata, VfsError>;
    /// 打开节点并返回一个文件操作句柄。
    fn open(&self) -> Result<Box<dyn File>, VfsError>;
    /// 在当前目录下查找名为 `name` 的子节点。
    fn lookup(&self, name: &str) -> Result<Arc<dyn VNode>, VfsError>;
    /// 在当前目录下创建名为 `name` 的子节点。
    fn create(&self, name: &str, typ: VNodeType) -> Result<Arc<dyn VNode>, VfsError>;
    /// 删除当前目录下的子节点。
    fn remove(&self, name: &str) -> Result<(), VfsError> {
        let _ = name;
        Err(VfsError::NotImplemented)
    }
    /// 读取符号链接的目标路径。只对 `VNodeType::SymLink` 类型有效。
    fn read_symlink(&self) -> Result<String, VfsError> {
        Err(VfsError::NotAFile) // 默认实现，非符号链接返回错误
    }
    /// 如果节点是设备，返回其对应的 `Device` 引用。
    fn as_device(&self) -> Option<&Device> {
        None
    }
}

/// 描述一个挂载点
struct MountPoint {
    path: String,
    mount_point_len: usize, // 优化路径匹配，存储 path 的长度
    root: Arc<dyn VNode>,
}

// --- VFS核心层 ---
pub struct Vfs {
    /// 全局根VNode，通常是一个内存文件系统
    root: Arc<dyn VNode>,
    /// 已挂载的文件系统列表
    mounts: Mutex<Vec<MountPoint>>,
    /// 已注册的文件系统驱动列表 (如 MemFs, Ext2Fs)
    fs_registry: RwLock<BTreeMap<&'static str, Arc<dyn FileSystem>>>,
}

impl Vfs {
    /// 创建一个新的VFS实例。
    /// 默认会挂载一个内存文件系统作为根目录。
    pub fn new() -> Self {
        let memfs = Arc::new(MemFs); // 假设 MemFs 是一个实现了 FileSystem trait 的结构体
        let root = memfs
            .mount(None, None)
            .expect("BUG: Could not mount memfs as root");
        let mut registry: BTreeMap<&'static str, Arc<dyn FileSystem>> = BTreeMap::new();
        registry.insert("memfs", memfs);
        Self {
            root,
            mounts: Mutex::new(Vec::new()),
            fs_registry: RwLock::new(registry),
        }
    }

    /// 注册一个文件系统驱动。
    pub fn register_fs(&self, fs: Arc<dyn FileSystem>) {
        self.fs_registry.write().insert(fs.fs_type(), fs);
    }

    /// 挂载一个文件系统到指定的挂载点。
    ///
    /// # Arguments
    /// * `device_str` - 可选的设备名称字符串 (如 "sda1")。
    /// * `mount_point` - VFS中用于挂载的路径 (如 "/mnt/data")。
    /// * `fs_type` - 文件系统类型字符串 (如 "ext2", "memfs")。
    /// * `args` - 可选的挂载参数。
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

        let device_manager = DEVICE_MANAGER.lock(); // 获取设备管理器锁
        let device = if let Some(dev) = device_str {
            device_manager.get_device(dev)
        } else {
            None
        };
        //drop(device_manager); // 在调用 fs.mount 之前释放设备管理器锁，防止潜在死锁

        let root_vnode = fs.mount(device, args)?;

        // 确保挂载点存在且是一个目录
        let parent_path = if mount_point == "/" {
            None // 根目录不能作为挂载点的父目录
        } else {
            mount_point.rsplit_once('/').map(|(p, _)| p) // 获取父目录路径
        };
        if let Some(parent_path_str) = parent_path {
            let parent_vnode = self.lookup(parent_path_str)?;
            if parent_vnode.node_type() != VNodeType::Dir {
                return Err(VfsError::NotADirectory);
            }
            // 确保挂载点本身不存在，或者如果存在，它是一个空目录 (可选的检查)
            // let existing_node = parent_vnode.lookup(mount_point.split('/').last().unwrap_or(""))
            // if existing_node.is_ok() { return Err(VfsError::AlreadyExists) }
        }

        let normalized_mount_point = mount_point.trim_matches('/').to_string();
        if normalized_mount_point.is_empty() {
            // 根目录只能被挂载一次，且在 Vfs::new() 中已完成
            return Err(VfsError::InvalidArgument);
        }

        self.mounts.lock().push(MountPoint {
            path: normalized_mount_point.clone(),
            mount_point_len: normalized_mount_point.len(),
            root: root_vnode,
        });

        Ok(())
    }

    /// 根据路径查找对应的VNode。会处理符号链接和挂载点。
    pub fn lookup(&self, path: &str) -> Result<Arc<dyn VNode>, VfsError> {
        self.path_to_vnode(path, 0)
    }

    /// 打开指定路径的文件。
    pub fn open(&self, path: &str) -> Result<Box<dyn File>, VfsError> {
        let vnode = self.lookup(path)?;
        if vnode.node_type() != VNodeType::File {
            return Err(VfsError::NotAFile);
        }
        vnode.open()
    }

    /// 创建一个文件。如果父目录不存在，会返回错误。
    pub fn create_file(&self, path: &str) -> Result<Arc<dyn VNode>, VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_vnode = self.lookup(parent_path)?;
        if parent_vnode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        parent_vnode.create(name, VNodeType::File)
    }

    /// 创建一个目录。如果父目录不存在，会返回错误。
    pub fn create_dir(&self, path: &str) -> Result<Arc<dyn VNode>, VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_vnode = self.lookup(parent_path)?;
        if parent_vnode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        parent_vnode.create(name, VNodeType::Dir)
    }

    /// 创建一个符号链接。
    pub fn create_symlink(
        &self,
        target_path: &str,
        link_path: &str,
    ) -> Result<Arc<dyn VNode>, VfsError> {
        // 符号链接的创建通常是在其父目录中，并且它的内容是目标路径。
        // `MemFs` 这样的文件系统需要支持 `create` 一个 `SymLink` 类型。
        // 在 `create` 方法中，对于 `SymLink` 类型，需要传入 `target_path` 作为额外参数。
        // 这里只是一个骨架，具体实现需要文件系统支持。
        let (parent_path, name) = self.split_path(link_path)?;
        let parent_vnode = self.lookup(parent_path)?;
        if parent_vnode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        // TODO: 这里需要一个更完善的 `create` 方法，允许传递符号链接的目标路径
        // 例如：parent_vnode.create_symlink(name, target_path)
        parent_vnode.create(name, VNodeType::SymLink)
    }

    /// 内部辅助函数：根据路径查找VNode，并处理符号链接循环。
    const MAX_SYMLINK_DEPTH: u32 = 8; // 最大符号链接解析深度

    fn path_to_vnode(&self, path: &str, depth: u32) -> Result<Arc<dyn VNode>, VfsError> {
        if depth > Self::MAX_SYMLINK_DEPTH {
            return Err(VfsError::MaxSymlinkDepth);
        }

        let path = path.trim_matches('/');
        if path.is_empty() {
            return Ok(self.root.clone()); // 根目录
        }

        // 1. 检查路径是否匹配某个挂载点
        let mounts = self.mounts.lock();
        // 优先匹配最长的挂载点路径
        if let Some(mount) = mounts
            .iter()
            .filter(|m| path.starts_with(&m.path))
            .max_by_key(|m| m.mount_point_len)
        {
            // 挂载点路径之后的部分
            let subpath = path[mount.mount_point_len..].trim_matches('/');
            if subpath.is_empty() {
                // 如果路径只是挂载点本身
                return self.resolve_symlink_if_needed(mount.root.clone(), depth);
            }
            // 从挂载点的根节点开始查找子路径
            let mut current = mount.root.clone();
            for component in subpath.split('/') {
                if component.is_empty() || component == "." {
                    continue;
                }
                if component == ".." {
                    // TODO: 处理 .. (需要获取父节点，这在VNode trait中未定义)
                    // 目前简单地跳过，实际需要向上遍历
                    return Err(VfsError::NotImplemented);
                }
                current = self.resolve_symlink_if_needed(current.lookup(component)?, depth)?;
            }
            return Ok(current);
        }

        // 2. 若无挂载点匹配，从全局根节点查找
        let mut current = self.root.clone();
        for component in path.split('/') {
            if component.is_empty() || component == "." {
                continue;
            }
            if component == ".." {
                // TODO: 处理 ..
                return Err(VfsError::NotImplemented);
            }
            current = self.resolve_symlink_if_needed(current.lookup(component)?, depth)?;
        }
        Ok(current)
    }

    /// 辅助函数：如果VNode是符号链接，则解析它。
    fn resolve_symlink_if_needed(
        &self,
        node: Arc<dyn VNode>,
        depth: u32,
    ) -> Result<Arc<dyn VNode>, VfsError> {
        if node.node_type() == VNodeType::SymLink {
            let target_path = node.read_symlink()?;
            // 递归调用 path_to_vnode 来解析符号链接的目标
            self.path_to_vnode(&target_path, depth + 1)
        } else {
            Ok(node)
        }
    }

    /// 辅助函数：将路径分割为 (父目录路径, 文件/目录名)。
    /// 例如 "/a/b/c" -> ("/a/b", "c")
    /// "/a" -> ("/", "a")
    /// "/" -> 报错
    fn split_path<'a>(&self, path: &'a str) -> Result<(&'a str, &'a str), VfsError> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Err(VfsError::EmptyPath);
        }
        if let Some((parent, name)) = path.rsplit_once('/') {
            // 如果 parent 是空字符串，说明是根目录下的文件，如 "a" -> ("", "a")
            // 此时父目录实际是 "/"
            Ok(((if parent.is_empty() { "/" } else { parent }), name))
        } else {
            // 没有斜杠，说明是根目录下的文件/目录
            Ok(("/", path))
        }
    }
}

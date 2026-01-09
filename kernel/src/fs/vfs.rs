use crate::drivers::{Device, DeviceError, DEVICE_MANAGER};
extern crate alloc;
use super::memfs::MemFs;
use alloc::format;
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
    pub static ref VFS: Vfs = Vfs::new();
}

/// VFS操作可能返回的错误类型
#[derive(Debug, PartialEq, Eq, Clone, Copy)] // 增加 Clone, Copy 便于错误处理
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
    /// 符号链接深度过深
    MaxSymlinkDepth,
    /// 文件系统类型不支持
    FsTypeNotSupported,
    /// 路径为空
    EmptyPath,
    /// 功能未实现
    NotImplemented,
    /// 目录非空
    DirectoryNotEmpty,
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
    /// 文件
    File,
    /// 目录
    Dir,
    /// 符号链接
    SymLink,
    /// 设备
    Device,
}

/// 文件或目录的元数据
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Metadata {
    /// 文件大小
    pub size: u64,
    /// UNIX权限位，如0o755
    pub permissions: u32,
    /// 用户ID
    pub uid: u32,
    /// 组ID
    pub gid: u32,
    /// 创建时间 (秒)
    pub ctime: u64,
    /// 最后修改时间 (秒)
    pub mtime: u64,
    /// 占用的块数
    pub blocks: u64,
    /// 硬链接数量
    pub nlinks: u64,
}

/// 文件操作接口
pub trait File: Send + Sync {
    /// 从文件当前位置读取数据到缓冲区。返回读取的字节数
    fn read(&self, buf: &mut [u8]) -> Result<usize, VfsError>;
    /// 将缓冲区数据写入文件当前位置。返回写入的字节数
    fn write(&mut self, buf: &[u8]) -> Result<usize, VfsError>;
    /// 移动文件指针到指定位置。返回新的文件指针位置
    fn seek(&self, pos: u64) -> Result<u64, VfsError>;
    /// 获取文件元数据
    fn stat(&self) -> Result<Metadata, VfsError>;
    /// 获取文件大小
    fn len(&self) -> Result<u64, VfsError> {
        self.stat().map(|m| m.size)
    }
    /// 截断文件到指定大小
    fn truncate(&mut self, size: u64) -> Result<(), VfsError> {
        let _ = size; // 默认实现什么也不做
        Err(VfsError::NotImplemented)
    }
    /// 文件系统操作
    fn ioctl(&self, op: u32, arg: usize) -> Result<usize, VfsError> {
        let _ = (op, arg); // 默认实现什么也不做
        Err(VfsError::NotImplemented)
    }
}

/// 文件系统实现接口
pub trait FileSystem: Send + Sync {
    /// 挂载文件系统。返回文件系统的根VNode。
    /// device: 实际设备（如块设备）。
    /// args: 挂载参数。
    fn mount(
        &self,
        device: Option<&Device>,
        args: Option<&[&str]>,
    ) -> Result<Arc<dyn VNode>, VfsError>;
    /// 返回文件系统类型的字符串标识符 (如"ext2")。
    fn fs_type(&self) -> &'static str;
}

/// 虚拟文件系统节点接口 (文件、目录、符号链接、设备)
pub trait VNode: Send + Sync + core::any::Any {
    /// 尝试将 VNode 转换为具体类型
    fn as_any(&self) -> &dyn core::any::Any;
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

    /// 在当前目录下创建名为 `name` 的符号链接，指向 `target_path`。
    fn create_symlink(&self, name: &str, target_path: &str) -> Result<Arc<dyn VNode>, VfsError> {
        let _ = (name, target_path);
        Err(VfsError::NotImplemented)
    }

    /// 创建名为 `name` 的目录
    fn create_dir(&self, name: &str) -> Result<Arc<dyn VNode>, VfsError> {
        let _ = name;
        Err(VfsError::NotImplemented)
    }
    /// 在当前目录下创建名为 `name` 的设备节点。
    /// major 和 minor 是设备号，`device_type` 是设备类型（例如 Block 或 Char）。
    fn create_device(
        &self,
        name: &str,
        major: u16,
        minor: u16,
        device_type: crate::drivers::DeviceType,
    ) -> Result<Arc<dyn VNode>, VfsError> {
        let _ = (name, major, minor, device_type); // 默认实现什么也不做
        Err(VfsError::NotImplemented)
    }

    /// 删除当前目录下的子节点。
    fn remove(&self, name: &str) -> Result<(), VfsError> {
        let _ = name;
        Err(VfsError::NotImplemented)
    }

    /// 在当前目录下重命名子节点。
    /// 旧名 -> 新名。
    fn rename(&self, old_name: &str, new_name: &str) -> Result<(), VfsError> {
        let _ = (old_name, new_name);
        Err(VfsError::NotImplemented)
    }
    /// 读取符号链接的目标路径。只对 `VNodeType::SymLink` 类型有效。
    fn read_symlink(&self) -> Result<String, VfsError> {
        Err(VfsError::NotAFile) // 默认实现，非符号链接返回错误
    }

    /// 列出目录中的条目。只对 `VNodeType::Dir` 类型有效。
    fn read_dir(&self) -> Result<Vec<String>, VfsError> {
        Err(VfsError::NotADirectory)
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

        let device_manager = DEVICE_MANAGER.read(); // 获取设备管理器锁
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
        let (parent_path, name) = self.split_path(link_path)?;
        let parent_vnode = self.lookup(parent_path)?;
        if parent_vnode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        parent_vnode.create_symlink(name, target_path)
    }

    /// 创建一个设备节点。
    pub fn create_device_node(
        &self,
        path: &str,
        major: u16,
        minor: u16,
        device_type: crate::drivers::DeviceType,
    ) -> Result<Arc<dyn VNode>, VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_vnode = self.lookup(parent_path)?;
        if parent_vnode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        parent_vnode.create_device(name, major, minor, device_type)
    }

    /// 删除指定路径的文件或目录。
    pub fn remove(&self, path: &str) -> Result<(), VfsError> {
        let (parent_path, name) = self.split_path(path)?;
        let parent_vnode = self.lookup(parent_path)?;
        if parent_vnode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        // 尝试删除，如果子节点是目录且非空，MemFs会返回 DirectoryNotEmpty
        parent_vnode.remove(name)
    }

    /// 读取指定目录下的所有条目。
    pub fn read_dir(&self, path: &str) -> Result<Vec<String>, VfsError> {
        let vnode = self.lookup(path)?;
        if vnode.node_type() != VNodeType::Dir {
            return Err(VfsError::NotADirectory);
        }
        vnode.read_dir()
    }

    /// 重命名或移动一个文件/目录 (mv old_path new_path)
    ///
    /// 支持：
    /// - 同一目录内重命名：mv /oldname /newname
    /// - 移动到不同目录：mv /dir1/oldname /dir2/newname
    /// - 同一目录内移动并重命名：mv /dir1/oldname /dir1/newname
    ///
    /// 当前实现：
    /// - MemFs 内支持同目录重命名和跨目录移动
    /// - 跨文件系统移动需要复制和删除，暂未实现
    pub fn rename_move(&self, old_path: &str, new_path: &str) -> Result<(), VfsError> {
        // 解析源路径
        let (old_parent_path, old_name) = self.split_path(old_path)?;
        let old_parent = self.lookup(old_parent_path)?;

        // 解析目标路径
        let (new_parent_path, new_name) = self.split_path(new_path)?;
        let new_parent = self.lookup(new_parent_path)?;

        // 检查源节点是否存在
        if old_parent.lookup(old_name).is_err() {
            return Err(VfsError::NotFound);
        }

        // 检查目标节点是否已存在
        if new_parent.lookup(new_name).is_ok() {
            return Err(VfsError::AlreadyExists);
        }

        // 检查源和目标是否在同一个目录
        if old_parent_path == new_parent_path {
            // 同一目录内，直接重命名
            old_parent.rename(old_name, new_name)
        } else {
            // 不同目录，需要移动
            // 尝试使用 MemFs 的 move_node 方法
            if let (Some(old_mem_parent), Some(new_mem_parent)) = (
                old_parent.as_any().downcast_ref::<super::memfs::MemVNode>(),
                new_parent.as_any().downcast_ref::<super::memfs::MemVNode>(),
            ) {
                // 都是 MemVNode，可以移动
                super::memfs::MemVNode::move_node(
                    old_mem_parent,
                    new_mem_parent,
                    old_name,
                    new_name,
                )
            } else {
                // 不是 MemFs，或者一个是 MemFs 一个不是
                // 跨文件系统移动需要复制和删除，暂未实现
                Err(VfsError::NotImplemented)
            }
        }
    }

    /// 内部辅助函数：根据路径查找VNode，并处理符号链接循环。
    const MAX_SYMLINK_DEPTH: u32 = 8; // 最大符号链接解析深度

    fn path_to_vnode(&self, path: &str, depth: u32) -> Result<Arc<dyn VNode>, VfsError> {
        if depth > Self::MAX_SYMLINK_DEPTH {
            return Err(VfsError::MaxSymlinkDepth);
        }

        // 规范化路径，处理 `.` 和 `..`
        let normalized_path = self.normalize_path(path);
        let path = normalized_path.trim_matches('/');

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
                // 注意：由于已经调用了 normalize_path，这里不会再遇到 ".."
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
            // 注意：由于已经调用了 normalize_path，这里不会再遇到 ".."
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

    pub fn walk(
        &self,
        start_path: &str,
    ) -> Result<Vec<(String, Vec<String>, Vec<String>)>, VfsError> {
        let mut results = Vec::new();
        // 使用一个栈来实现深度优先遍历
        let mut stack: Vec<String> = Vec::new();
        // 规范化起始路径并推入栈
        let normalized_start_path = self.normalize_path(start_path);
        stack.push(normalized_start_path);
        while let Some(current_dir_path) = stack.pop() {
            let current_vnode = self.lookup(&current_dir_path)?;
            if current_vnode.node_type() != VNodeType::Dir {
                // 如果不是目录，跳过并继续下一个（或者返回错误，取决于设计）
                // 这里我们选择跳过，因为它可能是一个文件或符号链接指向文件
                continue;
            }
            let entries = current_vnode.read_dir()?; // 获取当前目录的所有条目
            let mut dirnames: Vec<String> = Vec::new();
            let mut filenames: Vec<String> = Vec::new();
            let mut subdirs_to_visit: Vec<String> = Vec::new(); // 存储需要加入栈的子目录
            for entry_name in entries {
                let entry_path = if current_dir_path == "/" {
                    format!("/{}", entry_name)
                } else {
                    format!("{}/{}", current_dir_path, entry_name)
                };
                let entry_vnode = match self.lookup(&entry_path) {
                    Ok(node) => node,
                    Err(VfsError::NotFound) => {
                        // 条目可能在查找时被删除，或者是一个坏的符号链接，跳过
                        continue;
                    }
                    Err(e) => return Err(e), // 其他错误则直接返回
                };
                match entry_vnode.node_type() {
                    VNodeType::Dir => {
                        dirnames.push(entry_name.clone());
                        subdirs_to_visit.push(entry_path); // 将子目录路径加入待访问列表
                    }
                    VNodeType::File | VNodeType::SymLink | VNodeType::Device => {
                        filenames.push(entry_name.clone());
                    }
                }
            }
            // 按照 os.walk 的习惯，目录名和文件名是只包含名字，不含路径的
            // 并且我们返回的是当前目录的完整路径
            results.push((current_dir_path.clone(), dirnames, filenames));
            // 将所有子目录按照字典序（或其他稳定顺序）逆序加入栈，
            // 以保证在 `pop` 时实现深度优先的（正向）遍历顺序。
            // 例如：如果 dirnames 是 ["bar", "foo"]，逆序后为 ["foo", "bar"]。
            // 压栈后，先弹出 "bar" 遍历，再弹出 "foo" 遍历。
            subdirs_to_visit.sort_unstable_by(|a, b| b.cmp(a)); // 逆序排序，使得pop时是正序
            for subdir_path in subdirs_to_visit {
                stack.push(subdir_path);
            }
        }
        Ok(results)
    }
    /// 辅助函数：标准化路径，移除多余的斜杠，并确保以"/"开头但不会以"/"结尾（除非是根目录）。
    fn normalize_path(&self, path: &str) -> String {
        let parts: Vec<&str> = path
            .split('/')
            .filter(|&s| !s.is_empty() && s != ".") // 过滤空字符串和 "."
            .collect();
        let mut cleaned_parts = Vec::new();
        for part in parts.iter() {
            if *part == ".." {
                // 如果不是根目录，则弹出上一个组件，实现 ".."
                if let Some(last) = cleaned_parts.last_mut() {
                    if last != &"" {
                        // 避免弹出根目录
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

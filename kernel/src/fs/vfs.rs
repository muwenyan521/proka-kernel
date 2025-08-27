use core::hash::Hash;

use crate::drivers::{DEVICE_MANAGER, Device, DeviceError, DeviceOps};
extern crate alloc;
use super::memfs::MemFs;
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use spin::{Mutex, RwLock};

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
    MaxSymlinkDepth,
    FsTypeNotSupported,
    DeviceNotFound,
}

impl From<DeviceError> for VfsError {
    fn from(e: DeviceError) -> Self {
        VfsError::DeviceError(e)
    }
}

// --- 基础类型定义 ---
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VNodeType {
    File,
    Dir,
    SymLink,
    Device,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub size: u64,
    pub permissions: u32, // UNIX权限位
    pub uid: u32,
    pub gid: u32,
    pub ctime: u64,
    pub mtime: u64,
}

pub trait File: Send + Sync {
    fn read(&self, buf: &mut [u8]) -> Result<usize, VfsError>;
    fn write(&mut self, buf: &[u8]) -> Result<usize, VfsError>;
    fn seek(&self, pos: u64) -> Result<u64, VfsError>;
    fn stat(&self) -> Result<Metadata, VfsError>;
}

pub trait FileSystem: Send + Sync {
    fn mount(
        &self,
        device: Option<&Device>,
        args: Option<&[&str]>,
    ) -> Result<Arc<dyn VNode>, VfsError>;
    fn fs_type(&self) -> &'static str;
}

pub trait VNode: Send + Sync {
    fn node_type(&self) -> VNodeType; // 节点类型
    fn metadata(&self) -> Result<Metadata, VfsError>;
    fn open(&self) -> Result<Box<dyn File>, VfsError>;
    fn lookup(&self, name: &str) -> Result<Arc<dyn VNode>, VfsError>;
    fn create(&self, name: &str, typ: VNodeType) -> Result<Arc<dyn VNode>, VfsError>;
    fn as_device(&self) -> Option<Arc<dyn DeviceOps>> {
        None
    }
}

struct MountPoint {
    path: String,
    fs: Arc<dyn FileSystem>,
    root: Arc<dyn VNode>,
}

// --- VFS核心层 ---
pub struct Vfs {
    root: Arc<dyn VNode>,
    mounts: Mutex<Vec<MountPoint>>,
    fs_registry: RwLock<BTreeMap<&'static str, Arc<dyn FileSystem>>>,
}

impl Vfs {
    pub fn new() -> Self {
        let memfs = Arc::new(MemFs);
        let root = memfs.mount(None, None).unwrap();
        let mut registry: BTreeMap<&'static str, Arc<dyn FileSystem>> = BTreeMap::new();
        registry.insert("memfs", memfs);
        Self {
            root,
            mounts: Mutex::new(Vec::new()),
            fs_registry: RwLock::new(registry),
        }
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

        let device_manager = DEVICE_MANAGER.lock();
        let device = if let Some(dev) = device_str {
            device_manager.get_device(dev)
        } else {
            None
        };

        let root = fs.mount(device, args)?;

        // 在 `mounts` 修改前释放 `device_manager`（避免死锁）
        drop(device_manager);

        self.mounts.lock().push(MountPoint {
            path: mount_point.to_string(),
            fs: fs.clone(),
            root,
        });

        Ok(())
    }

    pub fn lookup(&self, path: &str) -> Result<Arc<dyn VNode>, VfsError> {
        let path = path.trim_matches('/');
        if path.is_empty() {
            return Ok(self.root.clone()); // 根目录直接返回
        }
        // 1. 检查路径是否匹配某个挂载点
        let mounts = self.mounts.lock();
        if let Some(mount) = mounts.iter().find(|m| path.starts_with(&m.path)) {
            // 截取挂载点之后的子路径（如 "/mnt/data" -> "data"）
            let subpath = path[mount.path.len()..].trim_matches('/');
            if subpath.is_empty() {
                return Ok(mount.root.clone()); // 直接访问挂载点根目录
            }
            // 从挂载点的根节点开始查找子路径
            let mut current = mount.root.clone();
            for component in subpath.split('/') {
                current = current.lookup(component)?;
            }
            return Ok(current);
        }
        // 2. 若无挂载点匹配，从全局根节点查找
        let mut current = self.root.clone();
        for component in path.split('/') {
            current = current.lookup(component)?;
        }
        Ok(current)
    }
}

//! 初始RAM磁盘（initrd）模块
//!
//! 此模块提供初始RAM磁盘（Initial RAM Disk, initrd）的加载和CPIO格式解析功能。
//! initrd是内核启动时加载到内存中的临时文件系统，包含启动过程中所需的驱动程序、工具和配置文件。
//!
//! ## 功能
//!
//! - 解析CPIO（newc格式）归档文件
//! - 将CPIO内容加载到虚拟文件系统（VFS）
//! - 自动创建目录结构和文件
//! - 支持常规文件、目录和符号链接
//!
//! ## CPIO格式
//!
//! 此模块支持CPIO的"newc"格式（也称为"new ASCII"格式），这是Linux initrd的标准格式。
//! CPIO归档由一系列文件条目组成，每个条目包含：
//! 1. 110字节的头部（包含文件元数据）
//! 2. 以null结尾的文件名
//! 3. 文件数据（4字节对齐）
//!
//! ## 文件类型支持
//!
//! - **常规文件**（CPIO_S_IFREG）：创建文件并写入数据
//! - **目录**（CPIO_S_IFDIR）：创建目录结构
//! - **符号链接**（CPIO_S_IFLNK）：创建符号链接
//!
//! ## 示例
//!
//! ```no_run
//! use kernel::libs::initrd;
//!
//! // 加载initrd（通常在启动过程中调用）
//! initrd::load_initrd();
//! ```
//!
//! ## 注意
//!
//! 此代码基于rcore-os的CPIO解析器，进行了适当的修改以适配本内核。
//! 原始代码：<https://github.com/rcore-os/cpio/blob/main/src/lib.rs>

extern crate alloc;
use crate::fs::memfs::MemFs;
use crate::fs::vfs::{File, FileSystem, VNodeType, VfsError, VFS};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use log::{debug, error, info, warn};

/// CPIO（newc格式）读取器
///
/// 此结构体提供对CPIO归档文件的迭代访问，每次迭代返回一个文件对象。
///
/// ## 生命周期
///
/// 读取器持有对原始字节数据的引用，因此其生命周期不能超过数据本身。
///
/// # 示例
///
/// ```rust,should_panic
/// use kernel::libs::initrd::CpioNewcReader;
///
/// let cpio_data = include_bytes!("initrd.cpio");
/// let reader = CpioNewcReader::new(cpio_data);
/// for obj in reader {
///     println!("{}", obj.unwrap().name);
/// }
/// ```
pub struct CpioNewcReader<'a> {
    buf: &'a [u8],
}

impl<'a> CpioNewcReader<'a> {
    /// 在缓冲区上创建新的CPIO读取器
    ///
    /// # 参数
    ///
    /// * `buf` - 包含CPIO归档数据的字节切片
    ///
    /// # 返回值
    ///
    /// 新的CPIO读取器实例
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }
}

/// CPIO文件中的文件系统对象
///
/// 表示CPIO归档中的一个文件或目录，包含元数据、名称和文件数据。
pub struct Object<'a> {
    /// 文件元数据
    pub metadata: Metadata,
    /// 完整路径名
    pub name: &'a str,
    /// 文件数据
    pub data: &'a [u8],
}

impl<'a> Iterator for CpioNewcReader<'a> {
    type Item = Result<Object<'a>, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: To workaround lifetime
        let s: &'a mut Self = unsafe { core::mem::transmute(self) };
        match inner(&mut s.buf) {
            Ok(Object {
                name: "TRAILER!!!", ..
            }) => None,
            res => Some(res),
        }
    }
}

/// 内部CPIO解析函数
///
/// 解析单个CPIO条目，更新缓冲区指针。
fn inner<'a>(buf: &'a mut &'a [u8]) -> Result<Object<'a>, ReadError> {
    const HEADER_LEN: usize = 110;
    const MAGIC_NUMBER: &[u8] = b"070701";

    if buf.len() < HEADER_LEN {
        return Err(ReadError::BufTooShort);
    }
    let magic = buf.read_bytes(6)?;
    if magic != MAGIC_NUMBER {
        return Err(ReadError::InvalidMagic);
    }
    let ino = buf.read_hex_u32()?;
    let mode = buf.read_hex_u32()?;
    let uid = buf.read_hex_u32()?;
    let gid = buf.read_hex_u32()?;
    let nlink = buf.read_hex_u32()?;
    let mtime = buf.read_hex_u32()?;
    let file_size = buf.read_hex_u32()?;
    let dev_major = buf.read_hex_u32()?;
    let dev_minor = buf.read_hex_u32()?;
    let rdev_major = buf.read_hex_u32()?;
    let rdev_minor = buf.read_hex_u32()?;
    let name_size = buf.read_hex_u32()? as usize;
    let _check = buf.read_hex_u32()?;
    let metadata = Metadata {
        ino,
        mode,
        uid,
        gid,
        nlink,
        mtime,
        file_size,
        dev_major,
        dev_minor,
        rdev_major,
        rdev_minor,
    };
    let name_with_nul = buf.read_bytes(name_size)?;
    if name_with_nul.last() != Some(&0) {
        return Err(ReadError::InvalidName);
    }
    let name = core::str::from_utf8(&name_with_nul[..name_size - 1])
        .map_err(|_| ReadError::InvalidName)?;
    buf.read_bytes(pad_to_4(HEADER_LEN + name_size))?;

    let data = buf.read_bytes(file_size as usize)?;
    buf.read_bytes(pad_to_4(file_size as usize))?;

    Ok(Object {
        metadata,
        name,
        data,
    })
}

/// 缓冲区扩展trait
///
/// 为字节切片提供十六进制解析和字节读取功能，用于CPIO格式解析。
///
/// ## 方法
///
/// - `read_hex_u32()`: 从缓冲区读取8个十六进制字符并解析为u32
/// - `read_bytes()`: 从缓冲区读取指定长度的字节
trait BufExt<'a> {
    fn read_hex_u32(&mut self) -> Result<u32, ReadError>;
    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ReadError>;
}

impl<'a> BufExt<'a> for &'a [u8] {
    fn read_hex_u32(&mut self) -> Result<u32, ReadError> {
        let (hex, rest) = self.split_at(8);
        *self = rest;
        let str = core::str::from_utf8(hex).map_err(|_| ReadError::InvalidASCII)?;
        let value = u32::from_str_radix(str, 16).map_err(|_| ReadError::InvalidASCII)?;
        Ok(value)
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ReadError> {
        if self.len() < len {
            return Err(ReadError::BufTooShort);
        }
        let (bytes, rest) = self.split_at(len);
        *self = rest;
        Ok(bytes)
    }
}

/// 计算填充到4字节对齐所需的字节数
///
/// CPIO格式要求所有字段和数据都按4字节对齐。此函数计算
/// 给定长度需要多少填充字节才能达到下一个4字节边界。
///
/// # 参数
///
/// * `len` - 当前长度（字节数）
///
/// # 返回值
///
/// 需要添加的填充字节数（0-3）
///
/// # 示例
///
/// ```
/// use kernel::libs::initrd::pad_to_4;
///
/// assert_eq!(pad_to_4(0), 0);
/// assert_eq!(pad_to_4(1), 3);
/// assert_eq!(pad_to_4(4), 0);
/// assert_eq!(pad_to_4(5), 3);
/// ```
fn pad_to_4(len: usize) -> usize {
    match len % 4 {
        0 => 0,
        x => 4 - x,
    }
}

/// CPIO读取错误类型
///
/// 表示在解析CPIO归档时可能发生的各种错误。
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    /// 无效的ASCII字符（非UTF-8或非十六进制字符）
    InvalidASCII,
    /// 无效的魔数（不是"070701"）
    InvalidMagic,
    /// 无效的文件名（不以null结尾或非UTF-8）
    InvalidName,
    /// 缓冲区太短，无法读取请求的数据
    BufTooShort,
}

/// 文件元数据
///
/// 表示CPIO归档中文件的元数据信息，对应CPIO头部的各个字段。
#[derive(Debug)]
pub struct Metadata {
    /// 索引节点号（inode number）
    pub ino: u32,
    /// 文件模式和类型
    pub mode: u32,
    /// 用户ID（owner user ID）
    pub uid: u32,
    /// 组ID（owner group ID）
    pub gid: u32,
    /// 硬链接计数
    pub nlink: u32,
    /// 修改时间（Unix时间戳）
    pub mtime: u32,
    /// 文件大小（字节数）
    pub file_size: u32,
    /// 设备主编号（major device number）
    pub dev_major: u32,
    /// 设备次编号（minor device number）
    pub dev_minor: u32,
    /// 特殊设备主编号（rdev major）
    pub rdev_major: u32,
    /// 特殊设备次编号（rdev minor）
    pub rdev_minor: u32,
}

// CPIO模式常量
/// 文件类型掩码
const CPIO_S_IFMT: u32 = 0o170000; // Mask for file type
/// 目录类型
const CPIO_S_IFDIR: u32 = 0o040000; // Directory
/// 常规文件类型
const CPIO_S_IFREG: u32 = 0o100000; // Regular file
/// 符号链接类型
const CPIO_S_IFLNK: u32 = 0o120000; // Symbolic link

/// 将初始RAM磁盘（initrd）加载到虚拟文件系统（VFS）中
///
/// 此函数解析作为原始字节提供的CPIO归档，提取其内容，
/// 并在VFS中重新创建文件和目录结构。
///
/// # 参数
///
/// * `initrd_data` - 包含CPIO归档数据的字节切片
///
/// # 返回值
///
/// 成功时返回`Ok(())`，失败时返回`VfsError`
///
/// # 错误
///
/// 此函数可能返回以下错误：
/// - `VfsError::IoError`: CPIO读取错误
/// - `VfsError::AlreadyExists`: 路径组件不是目录
/// - `VfsError::InvalidArgument`: 符号链接目标路径无效
/// - 其他VFS操作错误
///
/// # 处理流程
///
/// 1. 创建CPIO读取器遍历归档条目
/// 2. 跳过"TRAILER!!!"条目
/// 3. 规范化路径（添加前导斜杠，移除"./"前缀）
/// 4. 确保所有父目录存在
/// 5. 根据文件类型创建文件、目录或符号链接
pub fn load_cpio(initrd_data: &[u8]) -> Result<(), VfsError> {
    let reader = CpioNewcReader::new(initrd_data);
    let vfs = &*VFS;
    debug!("Loading CPIO archive...");
    for obj_result in reader {
        let obj = obj_result.map_err(|e| {
            error!("CPIO read error: {:?}", e);
            VfsError::IoError
        })?;

        let path = obj.name;
        if path == "TRAILER!!!" {
            continue; // Skip the trailer entry, already handled by iterator but good for explicit check
        }

        // Normalize path: CPIO paths are often like "foo/bar" or "./foo/bar".
        // We want absolute paths in VFS, e.g., "/foo/bar".
        let canonical_path = if path.starts_with('/') {
            path.to_string()
        } else if path.starts_with("./") {
            format!("/{}", &path[2..])
        } else {
            format!("/{}", path)
        };

        // Remove trailing slash unless it's the root itself.
        let final_path = if canonical_path.len() > 1 && canonical_path.ends_with('/') {
            canonical_path.trim_end_matches('/').to_string()
        } else {
            canonical_path
        };

        let node_type_mode = obj.metadata.mode & CPIO_S_IFMT;

        // Ensure all parent directories exist for the current object's path.
        // This loop iterates through path components and creates intermediate
        // directories if they don't already exist.
        let mut current_dir_segment = String::new();
        let components: Vec<&str> = final_path.split('/').filter(|&s| !s.is_empty()).collect();

        for (i, component) in components.iter().enumerate() {
            current_dir_segment.push('/');
            current_dir_segment.push_str(component);

            // If it's an intermediate component OR the last component is a directory itself,
            // ensure it exists and is a directory.
            if i < components.len() - 1 || node_type_mode == CPIO_S_IFDIR {
                match vfs.lookup(&current_dir_segment) {
                    Ok(node) => {
                        if node.node_type() != VNodeType::Dir {
                            error!(
                                "Path component '{}' for '{}' is not a directory!",
                                current_dir_segment, final_path
                            );
                            return Err(VfsError::AlreadyExists); // Or a specific error
                        }
                    }
                    Err(VfsError::NotFound) => {
                        vfs.create_dir(&current_dir_segment).map_err(|e| {
                            error!(
                                "Failed to create directory {}: {:?}",
                                current_dir_segment, e
                            );
                            e
                        })?;
                    }
                    Err(e) => {
                        error!("Error checking path {}: {:?}", current_dir_segment, e);
                        return Err(e);
                    }
                }
            }
        }
        debug!("Created parent directories for {}", final_path);
        // Now, handle the actual CPIO object based on its type
        match node_type_mode {
            CPIO_S_IFREG => {
                let file_inode = vfs.create_file(&final_path)?;
                debug!("Created file {}", final_path);
                let file_handle = File::new(file_inode);
                file_handle.write(obj.data)?;
            }
            CPIO_S_IFLNK => {
                let target_path =
                    core::str::from_utf8(obj.data).map_err(|_| VfsError::InvalidArgument)?;
                vfs.create_symlink(target_path, &final_path)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// 加载初始RAM磁盘（initrd）
///
/// 此函数是initrd模块的主要入口点，负责：
/// 1. 初始化内存文件系统（MemFs）作为根文件系统
/// 2. 从启动模块获取initrd数据
/// 3. 调用`load_cpio()`解析并加载CPIO归档
///
/// ## 启动模块集成
///
/// 此函数通过`crate::MODULE_REQUEST`获取启动时传递的模块信息。
/// 第一个模块被假定为initrd CPIO归档。
///
/// ## 日志输出
///
/// - 成功加载：输出"Initrd loaded successfully."
/// - 无initrd模块：输出警告"No initrd module found."
/// - 加载失败：输出错误"Failed to load initrd: {:?}"
/// - 模块请求失败：输出警告"Initrd module request failed."
///
/// ## 注意
///
/// 此函数通常在系统启动过程中调用一次。
pub fn load_initrd() {
    let memfs = MemFs;
    let root = memfs
        .mount(None, None)
        .expect("Failed to initialize root filesystem");
    VFS.init_root(root);

    // Load initrd
    if let Some(initrd_response) = crate::MODULE_REQUEST.get_response() {
        if let Some(inir) = initrd_response.modules().first() {
            unsafe {
                let slice: &[u8] = core::slice::from_raw_parts(inir.addr(), inir.size() as usize);
                match load_cpio(slice) {
                    Ok(_) => info!("Initrd loaded successfully."),
                    Err(e) => error!("Failed to load initrd: {:?}", e),
                }
            }
        } else {
            warn!("No initrd module found.");
        }
    } else {
        warn!("Initrd module request failed.");
    }
}

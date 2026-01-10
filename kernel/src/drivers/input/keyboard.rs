//! 键盘设备驱动程序
//!
//! 此模块提供了PS/2键盘的字符设备驱动程序。
//! 它实现了 [`CharDevice`] trait，允许从键盘读取按键输入。
//!
//! # 功能
//!
//! - 支持US 104键键盘布局
//! - 环形缓冲区存储输入字符
//! - 中断安全的操作（使用 `without_interrupts` 保护）
//! - 支持启用/禁用键盘输入
//! - 非阻塞读取（返回 [`DeviceError::WouldBlock`] 当无数据时）
//!
//! # 架构
//!
//! 键盘驱动程序使用两层结构：
//! 1. [`KeyboardInner`] - 内部状态，包含PC键盘解码器和环形缓冲区
//! 2. [`Keyboard`] - 外部接口，提供线程安全的访问
//!
//! # 示例
//!
//! ```rust
//! // 获取全局键盘实例
//! let keyboard = &*KEYBOARD;
//!
//! // 处理扫描码（通常在中断处理程序中调用）
//! keyboard.handle_scancode(0x1C); // Enter键
//!
//! // 读取按键输入
//! let mut buffer = [0u8; 16];
//! let bytes_read = keyboard.read(&mut buffer).unwrap_or(0);
//! ```
//!
//! # 注意
//!
//! 此实现仅支持读取操作（`read`），写入操作（`write`）返回 [`DeviceError::NotSupported`]。
//! 键盘设备是只读的字符设备。

extern crate alloc;
use crate::drivers::{CharDevice, Device, DeviceError, DeviceInner, DeviceType, SharedDeviceOps};
use alloc::string::String;
use alloc::sync::Arc;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard as PcKeyboard, ScancodeSet1};
use spin::Mutex;

/// 键盘输入缓冲区大小
///
/// 定义环形缓冲区可以存储的最大字符数。
/// 当缓冲区满时，新字符将被丢弃。
const BUFFER_SIZE: usize = 128;

/// 键盘内部状态结构体
///
/// 包含键盘解码器的实际状态和输入缓冲区。
/// 此结构体被 [`Mutex`] 保护以确保线程安全。
///
/// # 字段
///
/// - `pc_keyboard`: PC键盘解码器实例，处理扫描码到按键的转换
/// - `enabled`: 键盘是否启用输入
/// - `buffer`: 环形缓冲区，存储输入的字符
/// - `head`: 缓冲区读取位置
/// - `tail`: 缓冲区写入位置
pub struct KeyboardInner {
    pc_keyboard: PcKeyboard<layouts::Us104Key, ScancodeSet1>,
    enabled: bool,
    buffer: [char; BUFFER_SIZE],
    head: usize,
    tail: usize,
}

/// 键盘设备结构体
///
/// 提供线程安全的键盘设备接口。
/// 使用互斥锁保护内部状态，确保并发访问的安全性。
///
/// # 字段
///
/// - `inner`: 受互斥锁保护的内部状态
/// - `name`: 设备名称，固定为"keyboard"
pub struct Keyboard {
    inner: Mutex<KeyboardInner>,
    name: String,
}

impl Keyboard {
    /// 创建一个新的键盘设备实例
    ///
    /// 初始化键盘解码器（US 104键布局，扫描码集1）和空缓冲区。
    ///
    /// # 返回
    ///
    /// 返回初始化的 [`Keyboard`] 实例
    ///
    /// # 示例
    ///
    /// ```rust
    /// let keyboard = Keyboard::new();
    /// ```
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(KeyboardInner {
                pc_keyboard: PcKeyboard::new(
                    ScancodeSet1::new(),
                    layouts::Us104Key,
                    HandleControl::Ignore,
                ),
                enabled: true,
                buffer: ['\0'; BUFFER_SIZE],
                head: 0,
                tail: 0,
            }),
            name: String::from("keyboard"),
        }
    }

    /// 启用或禁用键盘输入
    ///
    /// 当键盘被禁用时，`handle_scancode` 将忽略所有扫描码。
    ///
    /// # 参数
    ///
    /// - `enabled`: 是否启用键盘输入
    ///
    /// # 注意
    ///
    /// 此操作在禁用中断的上下文中执行，确保原子性。
    pub fn set_enabled(&self, enabled: bool) {
        x86_64::instructions::interrupts::without_interrupts(|| {
            self.inner.lock().enabled = enabled;
        });
    }

    /// 检查键盘是否启用
    ///
    /// # 返回
    ///
    /// 如果键盘输入启用则返回 `true`，否则返回 `false`
    ///
    /// # 注意
    ///
    /// 此操作在禁用中断的上下文中执行，确保原子性。
    pub fn is_enabled(&self) -> bool {
        x86_64::instructions::interrupts::without_interrupts(|| {
            self.inner.lock().enabled
        })
    }

    /// 处理键盘扫描码
    ///
    /// 将扫描码转换为字符并存储到环形缓冲区中。
    /// 通常在键盘中断处理程序中调用此方法。
    ///
    /// # 参数
    ///
    /// - `scancode`: 键盘扫描码
    ///
    /// # 处理流程
    ///
    /// 1. 检查键盘是否启用
    /// 2. 将扫描码添加到键盘解码器
    /// 3. 处理按键事件
    /// 4. 将Unicode字符存储到环形缓冲区
    ///
    /// # 注意
    ///
    /// 如果缓冲区已满，新字符将被丢弃。
    pub fn handle_scancode(&self, scancode: u8) {
        let mut inner = self.inner.lock();
        if !inner.enabled {
            return;
        }

        if let Ok(Some(key_event)) = inner.pc_keyboard.add_byte(scancode) {
            if let Some(key) = inner.pc_keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => {
                        let tail = inner.tail;
                        let next_tail = (tail + 1) % BUFFER_SIZE;
                        if next_tail != inner.head {
                            inner.buffer[tail] = character;
                            inner.tail = next_tail;
                        }
                    }
                    DecodedKey::RawKey(_) => {}
                }
            }
        }
    }

    /// 创建键盘设备实例
    ///
    /// 使用全局 [`KEYBOARD`] 实例创建一个 [`Device`] 包装器。
    /// 设备号将自动分配。
    ///
    /// # 返回
    ///
    /// 返回封装好的 [`Device`] 实例，可以注册到设备管理器中
    ///
    /// # 示例
    ///
    /// ```rust
    /// let device = Keyboard::create_device();
    /// ```
    pub fn create_device() -> Device {
        Device::new_auto_assign(KEYBOARD.name.clone(), DeviceInner::Char(KEYBOARD.clone()))
    }
}

impl SharedDeviceOps for Keyboard {
    /// 获取设备名称
    ///
    /// 返回固定字符串"keyboard"
    fn name(&self) -> &str {
        &self.name
    }

    /// 获取设备类型
    ///
    /// 键盘设备始终返回 [`DeviceType::Char`]，表示字符设备
    fn device_type(&self) -> DeviceType {
        DeviceType::Char
    }

    /// 打开设备
    ///
    /// 对于键盘设备，打开操作总是成功
    ///
    /// # 返回
    ///
    /// 总是返回 `Ok(())`
    fn open(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// 关闭设备
    ///
    /// 对于键盘设备，关闭操作总是成功
    ///
    /// # 返回
    ///
    /// 总是返回 `Ok(())`
    fn close(&self) -> Result<(), DeviceError> {
        Ok(())
    }

    /// 执行I/O控制操作
    ///
    /// # 注意
    ///
    /// 键盘设备不支持I/O控制操作，总是返回 [`DeviceError::NotSupported`]
    ///
    /// # 参数
    ///
    /// - `_cmd`: 控制命令（未使用）
    /// - `_arg`: 命令参数（未使用）
    ///
    /// # 返回
    ///
    /// 总是返回 [`DeviceError::NotSupported`]
    fn ioctl(&self, _cmd: u64, _arg: u64) -> Result<u64, DeviceError> {
        Err(DeviceError::NotSupported)
    }
}

impl CharDevice for Keyboard {
    /// 从键盘读取数据
    ///
    /// 从环形缓冲区读取字符，转换为UTF-8字节序列。
    /// 如果缓冲区为空且请求的缓冲区大小大于0，返回 [`DeviceError::WouldBlock`]。
    ///
    /// # 参数
    ///
    /// - `buf`: 读取缓冲区，用于存储读取的字节
    ///
    /// # 返回
    ///
    /// - `Ok(usize)`: 成功读取的字节数
    /// - `Err(DeviceError::WouldBlock)`: 缓冲区为空且请求读取数据
    ///
    /// # 注意
    ///
    /// 此操作在禁用中断的上下文中执行，确保原子性。
    ///
    /// # 示例
    ///
    /// ```rust
    /// let keyboard = &*KEYBOARD;
    /// let mut buffer = [0u8; 16];
    /// match keyboard.read(&mut buffer) {
    ///     Ok(bytes_read) => println!("Read {} bytes: {:?}", bytes_read, &buffer[..bytes_read]),
    ///     Err(DeviceError::WouldBlock) => println!("No data available"),
    ///     Err(e) => println!("Error: {:?}", e),
    /// }
    /// ```
    fn read(&self, buf: &mut [u8]) -> Result<usize, DeviceError> {
        x86_64::instructions::interrupts::without_interrupts(|| {
            let mut inner = self.inner.lock();
            let mut read_count = 0;

            while read_count < buf.len() && inner.head != inner.tail {
                let c = inner.buffer[inner.head];
                inner.head = (inner.head + 1) % BUFFER_SIZE;

                let mut char_buf = [0u8; 4];
                let char_str = c.encode_utf8(&mut char_buf);
                let bytes = char_str.as_bytes();

                if read_count + bytes.len() <= buf.len() {
                    buf[read_count..read_count + bytes.len()].copy_from_slice(bytes);
                    read_count += bytes.len();
                } else {
                    inner.head = (inner.head + BUFFER_SIZE - 1) % BUFFER_SIZE;
                    break;
                }
            }

            if read_count == 0 && buf.len() > 0 {
                Err(DeviceError::WouldBlock)
            } else {
                Ok(read_count)
            }
        })
    }

    /// 向设备写入数据
    ///
    /// # 注意
    ///
    /// 键盘设备是只读设备，不支持写入操作。
    /// 总是返回 [`DeviceError::NotSupported`]。
    ///
    /// # 参数
    ///
    /// - `_buf`: 要写入的数据缓冲区（未使用）
    ///
    /// # 返回
    ///
    /// 总是返回 [`DeviceError::NotSupported`]
    fn write(&self, _buf: &[u8]) -> Result<usize, DeviceError> {
        Err(DeviceError::NotSupported)
    }

    /// 检查是否有可用数据
    ///
    /// 检查环形缓冲区是否包含待读取的字符。
    ///
    /// # 返回
    ///
    /// 如果缓冲区中有数据则返回 `true`，否则返回 `false`
    ///
    /// # 注意
    ///
    /// 此操作在禁用中断的上下文中执行，确保原子性。
    fn has_data(&self) -> bool {
        x86_64::instructions::interrupts::without_interrupts(|| {
            let inner = self.inner.lock();
            inner.head != inner.tail
        })
    }
}

lazy_static::lazy_static! {
    /// 全局键盘实例
    ///
    /// 使用懒加载静态变量创建全局共享的键盘实例。
    /// 这个实例在整个内核中共享，可以通过 `&*KEYBOARD` 访问。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use crate::drivers::input::keyboard::KEYBOARD;
    ///
    /// // 获取键盘实例
    /// let keyboard = &*KEYBOARD;
    ///
    /// // 处理扫描码
    /// keyboard.handle_scancode(0x1C);
    /// ```
    pub static ref KEYBOARD: Arc<Keyboard> = Arc::new(Keyboard::new());
}

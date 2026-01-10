//! Heap allocator module
//!
//! This module implements the heap allocator for the kernel.
//! It uses the `linked_list_allocator` crate to manage heap memory.

use talc::{ClaimOnOom, Span, Talc, Talck};
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

/// 堆的起始虚拟地址
///
/// 堆内存区域从虚拟地址 `0x_4444_4444_0000` 开始。
/// 这个地址选择在用户空间地址范围之外，以避免冲突。
pub const HEAP_START: usize = 0x_4444_4444_0000;

/// 堆的大小（字节数）
///
/// 当前堆大小为 8 MiB（8 * 1024 * 1024 字节）。
/// 这个大小可以根据内核需求进行调整。
pub const HEAP_SIZE: usize = 8 * 1024 * 1024;

/// 全局堆分配器
///
/// 使用 `talc` crate 提供的分配器，包装在自旋锁中以保证线程安全。
/// 分配器使用 `ClaimOnOom` 策略，在内存不足时尝试扩展堆区域。
///
/// ## 特性
///
/// - **线程安全**: 使用 `spin::Mutex` 保护内部状态
/// - **内存不足处理**: 使用 `ClaimOnOom` 策略
/// - **零大小初始化**: 初始时使用空区域，稍后通过 `init_heap` 函数声明实际堆区域
///
/// ## 注意
///
/// 在 `init_heap` 函数调用之前，此分配器无法分配内存。
/// 内核启动过程中应尽早调用 `init_heap` 来初始化堆。
#[global_allocator]
static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> = Talc::new(unsafe {
    // 如果在托管环境中，Rust运行时可能在main()调用之前进行分配，
    // 因此我们需要自动初始化区域
    ClaimOnOom::new(Span::empty())
})
.lock();

/// 初始化堆内存
///
/// 此函数映射堆内存区域并初始化全局分配器。
///
/// ## 处理流程
///
/// 1. 计算堆的起始和结束虚拟地址
/// 2. 将虚拟地址范围转换为页范围
/// 3. 为每个页分配物理帧并建立映射
/// 4. 将堆区域声明给全局分配器
///
/// # 参数
///
/// * `mapper` - 页表映射器，用于建立虚拟到物理的映射
/// * `frame_allocator` - 帧分配器，用于分配物理内存帧
///
/// # 返回值
///
/// * `Ok(())` - 成功初始化堆
/// * `Err(MapToError)` - 映射失败
///
/// # 错误
///
/// 可能返回以下错误：
/// - `MapToError::FrameAllocationFailed`: 无法分配物理帧
/// - 其他映射相关的错误
///
/// # 安全性
///
/// 此函数是不安全的，因为它直接操作页表和内存映射。
/// 调用者必须确保：
/// 1. 堆区域不与现有映射冲突
/// 2. 帧分配器提供有效的物理内存
/// 3. 映射器处于有效状态
///
/// # 示例
///
/// ```rust
/// use kernel::memory::allocator::init_heap;
/// use x86_64::structures::paging::Mapper;
///
/// // 假设已有mapper和frame_allocator
/// let result = init_heap(&mut mapper, &mut frame_allocator);
/// match result {
///     Ok(()) => println!("堆初始化成功"),
///     Err(e) => println!("堆初始化失败: {:?}", e),
/// }
/// ```
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let heap_start = VirtAddr::new(HEAP_START as u64);
    let heap_end = heap_start + HEAP_SIZE as u64;

    let page_range = {
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end - 1u64);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush();
        }
    }

    unsafe {
        ALLOCATOR
            .lock()
            .claim(Span::new(
                heap_start.as_mut_ptr::<u8>(),
                heap_end.as_mut_ptr::<u8>(),
            ))
            .expect("Failed to claim heap region");
    }

    Ok(())
}

use crate::memory::paging::frame::GLOBAL_FRAME_ALLOCATOR;
use log::debug;
use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{PageTableFlags, PhysFrame, PageTable},
};

pub fn init_page_table() {
    // 获取帧分配器
    let frame_allocator = GLOBAL_FRAME_ALLOCATOR.lock();
    debug!("No problem in frame alloc init");

    // 为各级页表分配物理帧
    let l4_frame = frame_allocator
        .alloc_frame()
        .expect("Failed to allocate L4 frame");
    let l3_frame = frame_allocator
        .alloc_frame()
        .expect("Failed to allocate L3 frame");
    let l2_frame = frame_allocator
        .alloc_frame()
        .expect("Failed to allocate L2 frame");
    let l1_frame = frame_allocator
        .alloc_frame()
        .expect("Failed to allocate L1 frame");

    // 获取页表的虚拟地址（假设恒等映射或已知映射）
    let l4_table_virt = phys_to_virt(l4_frame.start_address());
    let l3_table_virt = phys_to_virt(l3_frame.start_address());
    let l2_table_virt = phys_to_virt(l2_frame.start_address());
    let l1_table_virt = phys_to_virt(l1_frame.start_address());

    // 初始化页表内容
    unsafe {
        // 清零页表
        core::ptr::write_bytes(l4_table_virt.as_mut_ptr::<PageTable>(), 0, 4096);
        core::ptr::write_bytes(l3_table_virt.as_mut_ptr::<PageTable>(), 0, 4096);
        core::ptr::write_bytes(l2_table_virt.as_mut_ptr::<PageTable>(), 0, 4096);
        core::ptr::write_bytes(l1_table_virt.as_mut_ptr::<PageTable>(), 0, 4096);

        // 获取页表引用
        let l4_table: &mut PageTable = &mut *l4_table_virt.as_mut_ptr();
        let l3_table: &mut PageTable = &mut *l3_table_virt.as_mut_ptr();
        let l2_table: &mut PageTable = &mut *l2_table_virt.as_mut_ptr();
        let l1_table: &mut PageTable = &mut *l1_table_virt.as_mut_ptr();

        // 建立层级关系
        // L4 -> L3
        l4_table[0].set_addr(
            l3_frame.start_address(),
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        );

        // L3 -> L2
        l3_table[0].set_addr(
            l2_frame.start_address(),
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        );

        // L2 -> L1
        l2_table[0].set_addr(
            l1_frame.start_address(),
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        );

        // 现在映射物理帧到虚拟地址
        // 假设我们要映射前1MB物理内存到虚拟地址0x0
        for i in 0..256 {
            // 256个4KiB页 = 1MB
            let phys_addr = PhysAddr::new((i * 4096) as u64);
            let frame: PhysFrame = PhysFrame::containing_address(phys_addr);

            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

            l1_table[i].set_addr(frame.start_address(), flags);
        }
    }
}

// 辅助函数：物理地址到虚拟地址转换（需要根据你的内存映射实现）
fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    // 这里假设是恒等映射，实际情况可能不同
    VirtAddr::new(phys.as_u64())
}

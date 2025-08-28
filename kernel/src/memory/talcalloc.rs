use talc::*;
const HEAP_SIZE: usize = 8 * 1024 * 1024;
static mut ARENA: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

#[global_allocator]
static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> = Talc::new(unsafe {
    // if we're in a hosted environment, the Rust runtime may allocate before
    // main() is called, so we need to initialize the arena automatically
    ClaimOnOom::new(Span::from_array(core::ptr::addr_of!(ARENA).cast_mut()))
})
.lock();

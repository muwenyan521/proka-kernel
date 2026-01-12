pub mod pit;
pub mod tsc;

pub use tsc::{init, sleep_us, time_since_boot};

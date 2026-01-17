extern crate alloc;
use crate::memory::allocator::ALLOCATOR;
use alloc::{boxed::Box, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

/// Defintion of task state
pub enum TaskState {
    /// The init state, which means the task is ready to
    /// run by CPU.
    Ready,

    /// Sign the process is currently running.
    Running,

    /// If the process has completed running, sign it as
    /// terminated.
    Terminated,
}

/// The object of a task.
pub struct Task {
    /// The ID of this task.
    ///
    /// This uses type "u16", which means the task limit is
    /// 65535. (id range is 0~65535)
    id: u16,

    /// The state of the task.
    state: TaskState,

    /// The priority of the kernel (1-8)
    priority: u8,
}

impl Task {
    /// Create a new task object.
    pub fn new(id: u16, priority: u8) -> Box<Self> {
        Box::new(Self {
            id,
            state: TaskState::Ready,
            priority,
        })
    }

    /// Change the status of a task.
    pub fn update_stat(&mut self, new_state: TaskState) {
        self.state = new_state
    }
}

/// The task manager which contains lots of tasks.
pub struct TaskManager {
    /// The field which contains all tasks.
    tasks: Vec<Box<Task>>,

    /// The next task id
    next_tid: u16,
}

impl TaskManager {
    pub const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            next_tid: 0,
        }
    }

    pub fn create_task(&mut self, priority: u8) {
        // Allocate a task id
        let task_id = self.next_tid;
        
        // Push the task to the tasks container
        self.tasks.push(Task::new(task_id, priority));

        // Set up new ID
        self.next_tid = self.next_tid.wrapping_add(1);
    }
}


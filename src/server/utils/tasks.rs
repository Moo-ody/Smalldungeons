use crate::server::server::Server;

pub struct Task {
    pub run_in: u32,
    pub task_type: TaskType,
}

impl Task {
    pub const fn new(run_in: u32, task_type: TaskType) -> Self {
        Self {
            run_in,
            task_type
        }
    }

    pub fn run(self, server: &mut Server) {
        self.task_type.run(server)
    }
}

pub enum TaskType {
    MOVE(Box<dyn FnOnce(&mut Server)>),
    PTR(fn(&mut Server))
}

impl TaskType {
    pub fn run(self, server: &mut Server) {
        match self {
            Self::MOVE(f) => f(server),
            Self::PTR(f) => f(server)
        }
    }
}
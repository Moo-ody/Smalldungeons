use crate::server::server::Server;

pub struct Task {
    pub run_in: u32,
    pub callback: Box<dyn FnOnce(&mut Server)>,
}

impl Task {
    pub fn new(run_in: u32, task: impl FnOnce(&mut Server) + 'static) -> Self {
        Self {
            run_in,
            callback: Box::new(task)
        }
    }
}
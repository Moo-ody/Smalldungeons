use crate::server::entity::ai::ai_tasks::AiTasks;
use crate::server::entity::metadata::Metadata;

pub const ID: i8 = -1; // todo: see if players actually have one of these.

pub fn ai_tasks() -> Option<AiTasks> {
    None
}

pub fn metadata() -> Vec<Metadata> {
    Vec::new()
}
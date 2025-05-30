use crate::server::entity::ai::ai_enum::TaskType::WatchClosest;
use crate::server::entity::ai::ai_tasks::{AiTasks, TaskEntry};
use crate::server::entity::metadata::Metadata;
use crate::server::entity::metadata::Metadata::{IsChild, IsConverting, IsVillager};

pub const ID: i8 = 54;

pub fn ai_tasks() -> Option<AiTasks> {
    Some(AiTasks::create_from_entries(vec![TaskEntry::new(0, WatchClosest)])) // todo: more of the actual ai stuff
}

pub fn metadata() -> Vec<Metadata> {
    vec![IsChild(true), IsVillager(false), IsConverting(false)]
}
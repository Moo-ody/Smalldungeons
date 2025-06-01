use crate::server::entity::ai::ai_tasks::{AiTasks, TaskEntry};
use crate::server::entity::ai::TaskType::WatchClosest;
use crate::server::entity::metadata::EntityMetadata;

pub const ID: i8 = 54;

pub fn ai_tasks() -> Option<AiTasks> {
    Some(AiTasks::create_from_entries(vec![TaskEntry::new(0, WatchClosest)])) // todo: more of the actual ai stuff
}

pub const fn metadata() -> EntityMetadata {
    EntityMetadata::Zombie {
        is_child: true,
        is_villager: false,
        is_converting: false,
    }
}
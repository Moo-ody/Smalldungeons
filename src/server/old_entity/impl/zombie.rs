use crate::server::old_entity::ai::ai_tasks::{AiTasks, TaskEntry};
use crate::server::old_entity::ai::TaskType::WatchClosest;
use crate::server::old_entity::metadata::EntityMetadata;

pub const ID: i8 = 54;
pub const WIDTH: f32 = 0.6;
pub const HEIGHT: f32 = 1.95;

pub fn ai_tasks() -> Option<AiTasks> {
    Some(AiTasks::create_from_entries(&[TaskEntry::new(0, WatchClosest)])) // todo: more of the actual ai stuff
}

pub const fn metadata() -> EntityMetadata {
    EntityMetadata::Zombie {
        is_child: true,
        is_villager: false,
        is_converting: false,
    }
}
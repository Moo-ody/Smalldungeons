use crate::server::entity::ai::ai_tasks::AiTasks;
use crate::server::entity::metadata::EntityMetadata;

pub const ID: i8 = -1; // todo: not have this for players. they send a uuid instead.

pub const fn ai_tasks() -> Option<AiTasks> {
    None
}

pub const fn metadata() -> EntityMetadata {
    EntityMetadata::Player
}

pub const WIDTH: f32 = 0.6;
pub const HEIGHT: f32 = 1.8;
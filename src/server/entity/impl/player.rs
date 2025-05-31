use crate::server::entity::ai::ai_tasks::AiTasks;
use crate::server::entity::metadata::Metadata;
use std::collections::HashMap;

pub const ID: i8 = -1; // todo: not have this for players. they send a uuid instead.

pub fn ai_tasks() -> Option<AiTasks> {
    None
}

pub fn metadata() -> HashMap<i8, Metadata> {
    HashMap::new()
}
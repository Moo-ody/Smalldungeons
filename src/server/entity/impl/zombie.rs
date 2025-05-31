use crate::server::entity::ai::ai_tasks::{AiTasks, TaskEntry};
use crate::server::entity::ai::TaskType::WatchClosest;
use crate::server::entity::metadata::Metadata::{IsChild, IsConverting, IsVillager};
use crate::server::entity::metadata::{Metadata, METACHILDID, METACONVERTINGID, METAVILLAGERID};
use std::collections::HashMap;

pub const ID: i8 = 54;

pub fn ai_tasks() -> Option<AiTasks> {
    Some(AiTasks::create_from_entries(vec![TaskEntry::new(0, WatchClosest)])) // todo: more of the actual ai stuff
}

pub fn metadata() -> HashMap<i8, Metadata> {
    let mut data = HashMap::new();
    data.insert(METACHILDID, IsChild(true));
    data.insert(METAVILLAGERID, IsVillager(false));
    data.insert(METACONVERTINGID, IsConverting(false));
    data
}
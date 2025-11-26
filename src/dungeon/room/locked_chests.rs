use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

use crate::server::block::block_position::BlockPos;
use crate::server::utils::direction::Direction;

#[derive(Debug, Deserialize)]
struct LockedChestsFile {
    schema: u32,
    rooms: HashMap<String, Vec<LockedChestEntry>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LockedChestEntry {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub facing: String,
    pub lever: [i32; 3],
}

static LOCKED_CHESTS_DATA: Lazy<LockedChestsFile> = Lazy::new(|| {
    let locked_json = include_str!("../../room_data/Chests/locked.json");
    serde_json::from_str(locked_json).expect("Failed to parse locked.json")
});

/// Get locked chest entries for a specific room by name
pub fn get_room_locked_chests(room_name: &str) -> Option<&Vec<LockedChestEntry>> {
    LOCKED_CHESTS_DATA.rooms.get(room_name)
}

/// Convert facing string to Direction
pub fn facing_string_to_direction(facing: &str) -> Direction {
    match facing.to_lowercase().as_str() {
        "north" => Direction::North,
        "south" => Direction::South,
        "east" => Direction::East,
        "west" => Direction::West,
        _ => Direction::North, // Default to North if invalid
    }
}



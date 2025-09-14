use crate::server::block::block_position::BlockPos;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use include_dir::{include_dir, Dir};

// Include the superboomwalls data
const SUPERBOOMWALLS_DIR: Dir<'_> = include_dir!("src/room_data/relativecoords");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperboomWallBlock {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block_id: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperboomWallPattern {
    pub blocks: Vec<SuperboomWallBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSuperboomWallsEntry {
    pub crypts: Vec<Vec<SuperboomWallBlock>>, // array of walls, each a list of blocks
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperboomWallsFile {
    pub schema: u32,
    pub rooms: HashMap<String, RoomSuperboomWallsEntry>,
}

/// Load superboomwalls data from the wallsboom.json file
fn load_superboomwalls_data() -> SuperboomWallsFile {
    let data = SUPERBOOMWALLS_DIR.get_file("wallsboom.json")
        .expect("wallsboom.json not found")
        .contents_utf8()
        .expect("Invalid UTF-8 in wallsboom.json");
    
    serde_json::from_str(data)
        .expect("Failed to parse wallsboom.json")
}

/// Get superboomwalls patterns for a specific room
pub fn get_room_superboomwalls(shape: &str, room_name: &str) -> Option<Vec<SuperboomWallPattern>> {
    let file = load_superboomwalls_data();
    
    // Helper function to find entry by shape and room name
    fn find_entry<'a>(file: &'a SuperboomWallsFile, key: &str, _want_norm: &str) -> Option<&'a RoomSuperboomWallsEntry> {
        file.rooms.get(key)
    }
    
    // Try different key combinations
    let mut all_blocks: Vec<Vec<SuperboomWallBlock>> = Vec::new();
    
    // Try exact match first
    if let Some(entry) = find_entry(&file, room_name, shape) {
        all_blocks.extend(entry.crypts.iter().cloned());
    }
    
    // Try with shape prefix
    let shape_key = format!("{} {}", shape, room_name);
    if let Some(entry) = find_entry(&file, &shape_key, shape) {
        all_blocks.extend(entry.crypts.iter().cloned());
    }
    
    // Try with room name only
    if let Some(entry) = find_entry(&file, room_name, shape) {
        all_blocks.extend(entry.crypts.iter().cloned());
    }
    
    // Try with different case variations
    let lower_key = room_name.to_lowercase();
    if let Some(entry) = find_entry(&file, &lower_key, shape) {
        all_blocks.extend(entry.crypts.iter().cloned());
    }
    
    if all_blocks.is_empty() {
        return None;
    }
    
    let patterns = all_blocks.into_iter().map(|blocks| SuperboomWallPattern { blocks }).collect();
    Some(patterns)
}

/// Rotate a superboomwall block position based on room rotation
pub fn rotate_superboomwall_pos(block: &SuperboomWallBlock, rotation: crate::server::utils::direction::Direction) -> BlockPos {
    use crate::server::utils::direction::Direction;
    
    let (x, z) = match rotation {
        Direction::North => (block.x, block.z),
        Direction::East => (-block.z, block.x),
        Direction::South => (-block.x, -block.z),
        Direction::West => (block.z, -block.x),
        Direction::Up | Direction::Down => (block.x, block.z), // No rotation for vertical directions
    };
    
    BlockPos { x, y: block.y, z }
}

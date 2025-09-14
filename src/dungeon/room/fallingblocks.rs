use crate::server::block::block_position::BlockPos;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use include_dir::{include_dir, Dir};

// Include the fallingblocks data
const FALLINGBLOCKS_DIR: Dir<'_> = include_dir!("src/room_data/relativecoords");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallingBlock {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block_id: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallingBlockPattern {
    pub blocks: Vec<FallingBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomFallingBlocksEntry {
    pub crypts: Vec<Vec<FallingBlock>>, // array of falling block patterns, each a list of blocks
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallingBlocksFile {
    pub schema: u32,
    pub rooms: HashMap<String, RoomFallingBlocksEntry>,
}

/// Load fallingblocks data from the fallingblocks.json file
fn load_fallingblocks_data() -> FallingBlocksFile {
    let data = FALLINGBLOCKS_DIR.get_file("fallingblocks.json")
        .expect("fallingblocks.json not found")
        .contents_utf8()
        .expect("Invalid UTF-8 in fallingblocks.json");
    
    serde_json::from_str(data)
        .expect("Failed to parse fallingblocks.json")
}

/// Get falling block patterns for a specific room
pub fn get_room_fallingblocks(shape: &str, room_name: &str) -> Option<Vec<FallingBlockPattern>> {
    let file = load_fallingblocks_data();
    
    // Helper function to find entry by shape and room name
    fn find_entry<'a>(file: &'a FallingBlocksFile, key: &str, _want_norm: &str) -> Option<&'a RoomFallingBlocksEntry> {
        file.rooms.get(key)
    }
    
    // Try different key combinations
    let mut all_blocks: Vec<Vec<FallingBlock>> = Vec::new();
    
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
    
    let patterns = all_blocks.into_iter().map(|blocks| FallingBlockPattern { blocks }).collect();
    Some(patterns)
}

/// Rotate a falling block position based on room rotation
pub fn rotate_fallingblock_pos(block: &FallingBlock, rotation: crate::server::utils::direction::Direction) -> BlockPos {
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

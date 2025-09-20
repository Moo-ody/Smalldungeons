use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{DestroyEntites, EntityAttach, SpawnObject};
use crate::net::var_int::VarInt;
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

/// Entity implementation for falling floor blocks
/// Similar to DoorEntityImpl but for falling floor blocks
#[derive(Debug)]
pub struct FallingFloorEntityImpl {
    pub block: Blocks,
    distance_per_tick: f64,
    ticks_left: u32,
}

impl FallingFloorEntityImpl {
    pub fn new(block: Blocks, distance: f64, ticks: u32) -> Self {
        Self {
            block,
            distance_per_tick: distance / ticks as f64,
            ticks_left: ticks,
        }
    }
}

/// Offset for the falling block riding the bat entity
pub const FALLING_FLOOR_ENTITY_OFFSET: f64 = 0.65;

impl EntityImpl for FallingFloorEntityImpl {
    fn spawn(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        let world = entity.world_mut();
        let entity_id = world.new_entity_id();

        let object_data = {
            let block_state_id = self.block.get_block_state_id() as i32;
            let block_id = block_state_id >> 4;
            let metadata = block_state_id & 0b1111;
            block_id | (metadata << 12)
        };

        buffer.write_packet(&SpawnObject {
            entity_id: VarInt(entity_id),
            entity_variant: 70, // Falling block entity
            x: entity.position.x,
            y: entity.position.y + FALLING_FLOOR_ENTITY_OFFSET,
            z: entity.position.z,
            yaw: 0.0,
            pitch: 0.0,
            data: object_data,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
        });

        buffer.write_packet(&EntityAttach {
            entity_id,
            vehicle_id: entity.id,
            leash: false,
        });
    }

    fn despawn(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        buffer.write_packet(&DestroyEntites {
            entities: vec![VarInt(entity.id + 1)],
        });
    }

    fn tick(&mut self, entity: &mut Entity, _: &mut PacketBuffer) {
        entity.position.y -= self.distance_per_tick;
        self.ticks_left -= 1;
        if self.ticks_left == 0 {
            entity.world_mut().despawn_entity(entity.id);
        }
    }
}

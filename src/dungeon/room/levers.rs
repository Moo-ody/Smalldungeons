use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{DestroyEntites, EntityAttach, SpawnObject};
use crate::net::var_int::VarInt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy;

/// Room lever data structure for JSON parsing
#[derive(Debug, Deserialize)]
struct RoomLeverData {
    levers: Vec<LeverData>,
}

/// Lever file structure for JSON parsing
#[derive(Debug, Deserialize)]
struct LeverFile {
    schema: u32,
    rooms: HashMap<String, RoomLeverData>,
}

/// Load lever data from the JSON file
static LEVER_DATA: Lazy<LeverFile> = Lazy::new(|| {
    let lever_json = include_str!("../../room_data/lever shi/lever.json");
    serde_json::from_str(lever_json).expect("Failed to parse lever.json")
});

/// Get lever data for a specific room by shape and name (similar to crypts)
pub fn get_room_levers<'a>(_shape_key: &str, room_name: &str) -> Option<&'a Vec<LeverData>> {
    LEVER_DATA.rooms.get(room_name).map(|room_data| &room_data.levers)
}

/// Lever data structure for storing lever coordinates and associated falling blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeverData {
    /// The lever position [x, y, z]
    pub lever: [i32; 3],
    /// List of block positions that should fall when lever is activated
    pub blocks: Vec<[i32; 3]>,
}

/// Room data structure from JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomData {
    /// List of levers in this room
    pub levers: Vec<LeverData>,
    /// Crypts (empty for now)
    pub crypts: Vec<()>,
}

/// Root JSON structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeverJsonData {
    /// Schema version
    pub schema: u32,
    /// Map of room names to their data
    pub rooms: HashMap<String, RoomData>,
}

/// Lever system for managing lever interactions and falling blocks
pub struct LeverSystem {
    /// Map of room names to their lever data
    pub room_levers: HashMap<String, Vec<LeverData>>,
    /// Map of world positions to lever data
    pub world_levers: HashMap<BlockPos, LeverData>,
    /// Set of lever positions that have been activated
    pub activated_levers: HashSet<BlockPos>,
}

impl LeverSystem {
    pub fn new() -> Self {
        Self {
            room_levers: HashMap::new(),
            world_levers: HashMap::new(),
            activated_levers: HashSet::new(),
        }
    }

    /// Load lever data from JSON string
    pub fn load_from_json(&mut self, json_data: &str) -> Result<(), serde_json::Error> {
        let json_data: LeverJsonData = serde_json::from_str(json_data)?;
        
        // Convert the JSON structure to our internal format
        for (room_name, room_data) in json_data.rooms {
            self.room_levers.insert(room_name, room_data.levers);
        }
        Ok(())
    }

    /// Register all levers as interactable blocks in the world
    pub fn register_levers_as_interactable(&mut self, world: &mut World, rooms: &[crate::dungeon::room::room::Room]) {
        
        for (room_name, room_levers) in &self.room_levers {
            // Find the corresponding room to get its corner position
            if let Some(room) = rooms.iter().find(|r| r.room_data.name == *room_name) {
                let corner_pos = room.get_corner_pos();
                
                for lever in room_levers {
                    // Convert relative coordinates to world coordinates (same as crypts)
                    let relative_pos = BlockPos {
                        x: lever.lever[0],
                        y: lever.lever[1], // Y coordinates are absolute
                        z: lever.lever[2],
                    };
                    
                    // Rotate the position based on room rotation (same as crypts)
                    let rotated = relative_pos.rotate(room.rotation);
                    
                    // Convert to world coordinates (same as crypts)
                    let lever_pos = BlockPos {
                        x: corner_pos.x + rotated.x,
                        y: rotated.y, // Y coordinates are absolute
                        z: corner_pos.z + rotated.z,
                    };
                    
                    // Store the lever data with world coordinates
                    self.world_levers.insert(lever_pos.clone(), lever.clone());
                    
                    // Register the lever as an interactable block (lever blocks are already placed by room generation)
                    world.interactable_blocks.insert(lever_pos, crate::server::block::block_interact_action::BlockInteractAction::Lever);
                }
            }
        }
    }

    /// Get lever data for a specific room
    pub fn get_room_levers(&self, room_name: &str) -> Option<&Vec<LeverData>> {
        self.room_levers.get(room_name)
    }

    /// Find lever data by lever position
    pub fn find_lever_by_position(&self, lever_pos: &BlockPos) -> Option<&LeverData> {
        println!("Searching for lever at {:?}", lever_pos);
        if let Some(lever_data) = self.world_levers.get(lever_pos) {
            println!("  Found matching lever!");
            return Some(lever_data);
        }
        println!("  No matching lever found");
        None
    }

    /// Activate a lever and trigger falling blocks
    pub fn activate_lever(&mut self, world: &mut World, lever_pos: &BlockPos) -> bool {
        println!("Lever system: Attempting to activate lever at {:?}", lever_pos);
        
        // Check if lever has already been activated
        if self.activated_levers.contains(lever_pos) {
            println!("Lever already activated");
            return false;
        }
        
        // Find lever data first and clone it to avoid borrowing issues
        let lever_data = self.find_lever_by_position(lever_pos).cloned();
        if let Some(lever_data) = lever_data {
            println!("Found lever data with {} blocks", lever_data.blocks.len());
            // Mark lever as activated
            self.activated_levers.insert(*lever_pos);
            
            // Trigger falling blocks
            self.trigger_falling_blocks(world, &lever_data);
            true
        } else {
            println!("No lever data found for position {:?}", lever_pos);
            false
        }
    }

    /// Trigger falling blocks animation for a lever
    fn trigger_falling_blocks(&self, world: &mut World, lever_data: &LeverData) {
        let mut entities = Vec::new();
        
        // Process each block that should fall
        for block_coords in &lever_data.blocks {
            let x = block_coords[0];
            let y = block_coords[1];
            let z = block_coords[2];
            
            // Get the current block at this position
            let current_block = world.get_block_at(x, y, z);
            
            // Skip air blocks
            if matches!(current_block, Blocks::Air) {
                continue;
            }
            
            // Play RandomWoodClick sound 8 times with 250ms intervals from this block position
            for i in 0..8 {
                let delay_ticks = i * 5; // 250ms = 5 ticks at 20 TPS
                world.server_mut().schedule(delay_ticks, move |server| {
                    for (_, player) in &mut server.world.players {
                        let _ = player.write_packet(&crate::net::protocol::play::clientbound::SoundEffect {
                            sound: crate::server::utils::sounds::Sounds::RandomWoodClick.id(),
                            pos_x: x as f64 + 0.5,
                            pos_y: y as f64 + 0.5,
                            pos_z: z as f64 + 0.5,
                            volume: 2.0,
                            pitch: 0.49,
                        });
                    }
                });
            }
            
            // Replace with barrier block immediately
            world.set_block_at(Blocks::Barrier, x, y, z);
            world.interactable_blocks.remove(&BlockPos { x, y, z });
            
            // Schedule the barrier block to be replaced with air after 20 ticks
            world.server_mut().schedule(20, move |server| {
                server.world.set_block_at(Blocks::Air, x, y, z);
            });
            
            // Spawn falling block entity for animation
            let id = world.spawn_entity(
                DVec3::new(x as f64 + 0.5, y as f64 - LEVER_ENTITY_OFFSET, z as f64 + 0.5),
                {
                    let mut metadata = EntityMetadata::new(EntityVariant::Bat { hanging: false });
                    metadata.is_invisible = true;
                    metadata
                },
                LeverEntityImpl::new(current_block, 5.0, 20),
            ).unwrap();
            entities.push(id);
        }
    }
}

/// Entity implementation for lever falling blocks
/// Similar to DoorEntityImpl but for lever-triggered blocks
#[derive(Debug)]
pub struct LeverEntityImpl {
    pub block: Blocks,
    distance_per_tick: f64,
    ticks_left: u32,
}

impl LeverEntityImpl {
    pub fn new(block: Blocks, distance: f64, ticks: u32) -> Self {
        Self {
            block,
            distance_per_tick: distance / ticks as f64,
            ticks_left: ticks,
        }
    }
}

/// Offset for the falling block riding the bat entity
pub const LEVER_ENTITY_OFFSET: f64 = 0.65;

impl EntityImpl for LeverEntityImpl {
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
            y: entity.position.y + LEVER_ENTITY_OFFSET,
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
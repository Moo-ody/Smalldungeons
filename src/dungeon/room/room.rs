use crate::dungeon::crushers::Crusher;
use crate::dungeon::door::Door;
use crate::dungeon::dungeon::DUNGEON_ORIGIN;
use crate::dungeon::room::room_data::{RoomData, RoomShape, RoomType};
use crate::dungeon::room::crypts::{get_room_crypts, rotate_block_pos};
use crate::dungeon::room::mushroom::{get_room_mushrooms, MushroomSets};
use crate::dungeon::room::superboomwalls::{get_room_superboomwalls, SuperboomWallPattern, rotate_superboomwall_pos};
use crate::dungeon::room::fallingblocks::{get_room_fallingblocks, FallingBlockPattern, rotate_fallingblock_pos};
use crate::dungeon::room::levers::{get_room_levers, LeverData};
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::utils::direction::Direction;
use crate::server::world::World;
use std::collections::HashSet;

#[derive(Debug)]
pub struct RoomSegment {
    pub x: usize,
    pub z: usize,
    pub neighbours: [Option<RoomNeighbour>; 4]
}

#[derive(Debug, Clone, Copy)]
pub struct RoomNeighbour {
    pub door_index: usize,
    pub room_index: usize,
}

#[derive(Debug)]
pub struct Room {
    pub segments: Vec<RoomSegment>,
    pub room_data: RoomData,
    pub rotation: Direction,

    pub tick_amount: u32,
    pub crushers: Vec<Crusher>,
    pub crypt_patterns: Vec<Vec<(BlockPos, Option<u16>)>>, // world positions with expected block ids
    pub crypts_checked: bool,
    pub crypts_detected_count: usize,
    pub superboomwall_patterns: Vec<SuperboomWallPattern>, // superboomwall patterns for this room
    pub superboomwalls_checked: bool,
    pub superboomwalls_detected_count: usize,
    pub fallingblock_patterns: Vec<FallingBlockPattern>, // falling block patterns for this room
    pub fallingblocks_checked: bool,
    pub fallingblocks_detected_count: usize,
    pub scheduled_falling_removals: Vec<(u64, Vec<crate::dungeon::room::fallingblocks::FallingBlock>)>, // (tick, blocks)
    pub mushroom_sets: Vec<MushroomSets>,
    pub lever_data: Vec<LeverData>, // Store lever data for this room
    
    pub entered: bool,
}

impl Room {

    pub fn new(
        mut segments: Vec<RoomSegment>,
        dungeon_doors: &[Door],
        room_data: RoomData
    ) -> Room {
        // Sort room segments by z and then x
        segments.sort_by(|a, b| a.z.cmp(&b.z));
        segments.sort_by(|a, b| a.x.cmp(&b.x));
        
        let rotation = Room::get_rotation_from_segments(&segments, dungeon_doors);
        let corner_pos = Room::get_corner_pos_from(&segments, &rotation, &room_data);

        let crushers = room_data.crusher_data.iter().map(|data| {
            let mut crusher = Crusher::from_json(data);
            
            crusher.direction = crusher.direction.rotate(rotation);
            crusher.block_pos = crusher.block_pos.rotate(rotation);

            // This is fucking aids
            match rotation {
                Direction::North => match crusher.direction {
                    Direction::East | Direction::West => crusher.block_pos.add_z(crusher.width - 1),
                    _ => crusher.block_pos.add_x(crusher.width - 1),
                },
                Direction::South => match crusher.direction {
                    Direction::East | Direction::West => crusher.block_pos.add_z(-crusher.width + 1),
                    _ => crusher.block_pos.add_x(-crusher.width + 1),
                }
                _ => crusher.block_pos,
            };

            crusher.block_pos = crusher.block_pos
                .add_x(corner_pos.x)
                .add_z(corner_pos.z);

            crusher
        }).collect::<Vec<Crusher>>();

        // Build crypt patterns from relative coords json
        let mut crypt_patterns: Vec<Vec<(BlockPos, Option<u16>)>> = Vec::new();

        let shape_key = match room_data.shape {
            RoomShape::OneByOne => "1x1",
            RoomShape::OneByOneEnd => "1x1_E",
            RoomShape::OneByOneCross => "1x1_X",
            RoomShape::OneByOneStraight => "1x1_I",
            RoomShape::OneByOneBend => "1x1_L",
            RoomShape::OneByOneTriple => "1x1_3",
            RoomShape::OneByTwo => "1x2",
            _ => "rest",
        };

        if let Some(rc) = get_room_crypts(shape_key, &room_data.name) {
            for pattern in rc.patterns {
                let mut world_blocks: Vec<(BlockPos, Option<u16>)> = Vec::new();
                for blk in pattern.blocks {
                    let rotated = rotate_block_pos(&blk, rotation);
                    // Room block data is placed at its original absolute Y levels,
                    // so crypt coordinates use their absolute Y directly.
                    let world_pos = BlockPos { x: corner_pos.x + rotated.x, y: blk.y, z: corner_pos.z + rotated.z };
                    world_blocks.push((world_pos, blk.block_id));
                }
                crypt_patterns.push(world_blocks);
            }
        }

        // Build mushroom secret sets
        let mushroom_sets = get_room_mushrooms(&room_data.name, rotation, &corner_pos);

        // Build superboomwall patterns from relative coords json
        let mut superboomwall_patterns = Vec::new();
        if let Some(patterns) = get_room_superboomwalls(shape_key, &room_data.name) {
            for pattern in patterns {
                let mut world_blocks = Vec::new();
                for block in pattern.blocks {
                    // Convert relative coordinates to world coordinates
                    let rotated = rotate_superboomwall_pos(&block, rotation);
                    let world_pos = BlockPos { 
                        x: corner_pos.x + rotated.x, 
                        y: block.y, // Use absolute Y like crypts
                        z: corner_pos.z + rotated.z 
                    };
                    world_blocks.push(crate::dungeon::room::superboomwalls::SuperboomWallBlock {
                        x: world_pos.x,
                        y: world_pos.y,
                        z: world_pos.z,
                        block_id: block.block_id,
                    });
                }
                superboomwall_patterns.push(crate::dungeon::room::superboomwalls::SuperboomWallPattern {
                    blocks: world_blocks,
                });
            }
        }

        // Build falling block patterns from relative coords json
        let mut fallingblock_patterns = Vec::new();
        if let Some(patterns) = get_room_fallingblocks(shape_key, &room_data.name) {
            for pattern in patterns {
                let mut world_blocks = Vec::new();
                for block in pattern.blocks {
                    // Convert relative coordinates to world coordinates
                    let rotated = rotate_fallingblock_pos(&block, rotation);
                    let world_pos = BlockPos { 
                        x: corner_pos.x + rotated.x, 
                        y: block.y, // Use absolute Y like crypts
                        z: corner_pos.z + rotated.z 
                    };
                    world_blocks.push(crate::dungeon::room::fallingblocks::FallingBlock {
                        x: world_pos.x,
                        y: world_pos.y,
                        z: world_pos.z,
                        block_id: block.block_id,
                    });
                }
                fallingblock_patterns.push(crate::dungeon::room::fallingblocks::FallingBlockPattern {
                    blocks: world_blocks,
                });
            }
        }

        // Store lever data for this room (similar to crypts and superboom walls)
        let mut lever_data = Vec::new();
        if let Some(room_levers) = get_room_levers(shape_key, &room_data.name) {
            for lever_data_item in room_levers {
                // Convert relative coordinates to world coordinates (same as crypts)
                let relative_pos = BlockPos {
                    x: lever_data_item.lever[0],
                    y: lever_data_item.lever[1], // Y coordinates are absolute
                    z: lever_data_item.lever[2],
                };
                
                // Rotate the position based on room rotation (same as crypts)
                let rotated = relative_pos.rotate(rotation);
                
                // Convert to world coordinates (same as crypts)
                let lever_pos = BlockPos {
                    x: corner_pos.x + rotated.x,
                    y: rotated.y, // Y coordinates are absolute
                    z: corner_pos.z + rotated.z,
                };
                
                // Store the lever data with world coordinates
                let mut world_lever_data = lever_data_item.clone();
                world_lever_data.lever = [lever_pos.x, lever_pos.y, lever_pos.z];
                
                // Convert block positions to world coordinates as well
                let mut world_blocks = Vec::new();
                for block_pos in &lever_data_item.blocks {
                    let relative_block_pos = BlockPos {
                        x: block_pos[0],
                        y: block_pos[1], // Y coordinates are absolute
                        z: block_pos[2],
                    };
                    
                    // Rotate the block position based on room rotation (same as crypts)
                    let rotated_block_pos = relative_block_pos.rotate(rotation);
                    
                    // Convert to world coordinates (same as crypts)
                    let world_block_pos = BlockPos {
                        x: corner_pos.x + rotated_block_pos.x,
                        y: rotated_block_pos.y, // Y coordinates are absolute
                        z: corner_pos.z + rotated_block_pos.z,
                    };
                    
                    world_blocks.push([world_block_pos.x, world_block_pos.y, world_block_pos.z]);
                }
                world_lever_data.blocks = world_blocks;
                
                lever_data.push(world_lever_data);
            }
        }

        Room {
            segments,
            room_data,
            rotation,
            tick_amount: 0,
            crushers,
            crypt_patterns,
            crypts_checked: false,
            crypts_detected_count: 0,
            superboomwall_patterns,
            superboomwalls_checked: false,
            superboomwalls_detected_count: 0,
            fallingblock_patterns,
            fallingblocks_checked: false,
            fallingblocks_detected_count: 0,
            scheduled_falling_removals: Vec::new(),
            mushroom_sets,
            lever_data,
            entered: false,
        }
    }

    pub fn get_corner_pos(&self) -> BlockPos {
        Room::get_corner_pos_from(&self.segments, &self.rotation, &self.room_data)
    }

    pub fn get_corner_pos_from(segments: &[RoomSegment], rotation: &Direction, room_data: &RoomData) -> BlockPos {
        let min_x = segments.iter().min_by(|a, b| a.x.cmp(&b.x)).unwrap().x;
        let min_z = segments.iter().min_by(|a, b| a.z.cmp(&b.z)).unwrap().z;

        // Special handling for bossrooms - commented out
        // if room_data.room_type == crate::dungeon::room::room_data::RoomType::Boss {
        //     match rotation {
        //         Direction::North => BlockPos { x: -8, y: 254, z: -8 },
        //         Direction::East => BlockPos { x: -8 + room_data.length - 1, y: 254, z: -8 },
        //         Direction::South => BlockPos { x: -8 + room_data.length - 1, y: 254, z: -8 + room_data.width - 1 },
        //         Direction::West => BlockPos { x: -8, y: 254, z: -8 + room_data.width - 1 },
        //         _ => unreachable!(),
        //     }
        // } else {
        let x = min_x as i32 * 32 + DUNGEON_ORIGIN.0;
        let y = 68;
        let z = min_z as i32 * 32 + DUNGEON_ORIGIN.1;
        
        match rotation {
            Direction::North => BlockPos { x, y, z },
            Direction::East => BlockPos { x: x + room_data.length - 1, y, z },
            Direction::South => BlockPos { x: x + room_data.length - 1, y, z: z + room_data.width - 1 },
            Direction::West => BlockPos { x, y, z: z + room_data.width - 1 },
            _ => unreachable!(),
        }
        // }
    }

    pub fn tick(&mut self, world: &mut World) {
        self.tick_amount += 1;
        
        // Process scheduled falling block removals
        self.process_scheduled_falling_removals(world);
    }

    pub fn detect_crypts(&mut self, world: &World) -> usize {
        if self.crypts_checked {
            return self.crypts_detected_count;
        }
        let mut detected = 0usize;
        'pattern: for pattern in &self.crypt_patterns {
            for (pos, expected) in pattern {
                if let Some(block_id) = expected {
                    let state_id = world.get_block_at(pos.x, pos.y, pos.z).get_block_state_id();
                    let id = (state_id >> 4) as u16;
                    if id != *block_id {
                        continue 'pattern;
                    }
                }
            }
            detected += 1;
        }
        self.crypts_detected_count = detected;
        self.crypts_checked = true;
        detected
    }

    pub fn debug_crypt_mismatch(&self, world: &World) {
        if self.crypt_patterns.is_empty() { return; }
        let pattern = &self.crypt_patterns[0];
        println!("[crypts] debug first pattern for '{}' ({} blocks):", self.room_data.name, pattern.len());
        for (i, (pos, expected)) in pattern.iter().enumerate().take(12) {
            let state_id = world.get_block_at(pos.x, pos.y, pos.z).get_block_state_id();
            let id = (state_id >> 4) as u16;
            println!(
                "[crypts]   #{} at ({}, {}, {}): world_id={} expected={:?}",
                i, pos.x, pos.y, pos.z, id, expected
            );
        }
    }

    /// Explodes (removes) all crypt patterns that have any block within `radius`
    /// of `center`. Returns the number of crypts exploded.
    pub fn explode_crypt_near(&mut self, world: &mut World, center: &BlockPos, radius: i32) -> usize {
        if self.crypt_patterns.is_empty() { return 0; }

        // Collect indices to remove to avoid borrow issues while mutating
        let mut indices: Vec<usize> = Vec::new();
        for (i, pattern) in self.crypt_patterns.iter().enumerate() {
            let in_range = pattern.iter().any(|(pos, _)| {
                let dx = (pos.x - center.x).abs();
                let dy = (pos.y - center.y).abs();
                let dz = (pos.z - center.z).abs();
                dx.max(dy).max(dz) <= radius
            });
            if in_range { indices.push(i); }
        }

        if indices.is_empty() { return 0; }

        // Remove from highest to lowest index to keep indices valid
        indices.sort_unstable_by(|a, b| b.cmp(a));
        let mut exploded = 0usize;
        for idx in indices {
            if let Some(pattern) = self.crypt_patterns.get(idx).cloned() {
                for (pos, _) in pattern.into_iter() {
                    world.set_block_at(Blocks::Air, pos.x, pos.y, pos.z);
                }
                // Actually remove the pattern after applying blocks
                let _ = self.crypt_patterns.remove(idx);
                exploded += 1;
            }
        }

        exploded
    }

    /// Detect superboomwalls in the room (similar to crypts)
    pub fn detect_superboomwalls(&mut self, world: &World) -> usize {
        if self.superboomwalls_checked {
            return self.superboomwalls_detected_count;
        }
        let mut detected = 0usize;
        'pattern: for pattern in &self.superboomwall_patterns {
            for block in &pattern.blocks {
                let pos = BlockPos { x: block.x, y: block.y, z: block.z };
                let state_id = world.get_block_at(pos.x, pos.y, pos.z).get_block_state_id();
                let id = (state_id >> 4) as u16;
                if id != block.block_id {
                    continue 'pattern;
                }
            }
            detected += 1;
        }
        self.superboomwalls_detected_count = detected;
        self.superboomwalls_checked = true;
        detected
    }

    /// Explodes (removes) ONLY THE FIRST superboomwall pattern that has any block within `radius`
    /// of `center`. Returns the number of walls exploded (0 or 1).
    /// This is the key difference from crypts - only one wall can be exploded at a time.
    pub fn explode_superboomwall_near(&mut self, world: &mut World, center: &BlockPos, radius: i32) -> usize {
        if self.superboomwall_patterns.is_empty() { 
            return 0; 
        }

        // Find the FIRST pattern that has any block within range
        for (i, pattern) in self.superboomwall_patterns.iter().enumerate() {
            let in_range = pattern.blocks.iter().any(|block| {
                let pos = BlockPos { x: block.x, y: block.y, z: block.z };
                let dx = (pos.x - center.x).abs();
                let dy = (pos.y - center.y).abs();
                let dz = (pos.z - center.z).abs();
                dx.max(dy).max(dz) <= radius
            });
            
            if in_range {
                // Found the first wall in range - explode it and return
                if let Some(pattern) = self.superboomwall_patterns.get(i).cloned() {
                    for block in pattern.blocks {
                        let pos = BlockPos { x: block.x, y: block.y, z: block.z };
                        world.set_block_at(Blocks::Air, pos.x, pos.y, pos.z);
                    }
                    // Remove the pattern after applying blocks
                    let _ = self.superboomwall_patterns.remove(i);
                    return 1; // Only one wall exploded
                }
            }
        }

        0 // No walls in range
    }

    /// Detect falling blocks in the room (similar to crypts)
    pub fn detect_fallingblocks(&mut self, world: &World) -> usize {
        if self.fallingblocks_checked {
            return self.fallingblocks_detected_count;
        }
        let mut detected = 0usize;
        'pattern: for pattern in &self.fallingblock_patterns {
            for block in &pattern.blocks {
                let pos = BlockPos { x: block.x, y: block.y, z: block.z };
                let state_id = world.get_block_at(pos.x, pos.y, pos.z).get_block_state_id();
                let id = (state_id >> 4) as u16;
                if id != block.block_id {
                    continue 'pattern;
                }
            }
            detected += 1;
        }
        self.fallingblocks_detected_count = detected;
        self.fallingblocks_checked = true;
        detected
    }

    /// Check if player is touching any falling block and trigger the fall
    /// Returns true if a falling block pattern was triggered
    pub fn check_fallingblocks_collision(&mut self, world: &mut World, player_pos: &BlockPos) -> bool {
        if self.fallingblock_patterns.is_empty() {
            return false;
        }

        // Check if player is standing on any falling block
        let player_feet_pos = BlockPos { 
            x: player_pos.x, 
            y: player_pos.y - 1, // Check block below player's feet
            z: player_pos.z 
        };

        // Find the first pattern that has the block the player is standing on
        for (i, pattern) in self.fallingblock_patterns.iter().enumerate() {
            let is_standing_on = pattern.blocks.iter().any(|block| {
                let pos = BlockPos { x: block.x, y: block.y, z: block.z };
                pos == player_feet_pos
            });
            
            if is_standing_on {
                // Player is standing on this falling block pattern - trigger it
                self.trigger_fallingblocks(world, i);
                return true;
            }
        }

        false
    }

    /// Trigger falling blocks for a specific pattern index
    fn trigger_fallingblocks(&mut self, world: &mut World, pattern_index: usize) {
        if let Some(pattern) = self.fallingblock_patterns.get(pattern_index).cloned() {
            // Play sound effect
            for (_, player) in &mut world.players {
                let _ = player.write_packet(&crate::net::protocol::play::clientbound::SoundEffect {
                    sound: crate::server::utils::sounds::Sounds::RandomFizz.id(),
                    pos_x: pattern.blocks[0].x as f64 + 0.5,
                    pos_y: pattern.blocks[0].y as f64 + 0.5,
                    pos_z: pattern.blocks[0].z as f64 + 0.5,
                    volume: 5.0,
                    pitch: 0.49,
                });
            }

            // Spawn falling block entities for each block in the pattern
            for block in &pattern.blocks {
                let x = block.x;
                let y = block.y;
                let z = block.z;
                
                // Get the current block at this position
                let current_block = world.get_block_at(x, y, z);
                
                // Skip air blocks
                if matches!(current_block, crate::server::block::blocks::Blocks::Air) {
                    continue;
                }
                
                // Replace with barrier block immediately
                world.set_block_at(crate::server::block::blocks::Blocks::Barrier, x, y, z);
                world.interactable_blocks.remove(&crate::server::block::block_position::BlockPos { x, y, z });
                
                // Schedule the barrier block to be replaced with air after 20 ticks
                world.server_mut().schedule(20, move |server| {
                    server.world.set_block_at(crate::server::block::blocks::Blocks::Air, x, y, z);
                });
                
                // Spawn falling block entity for animation
                let _ = world.spawn_entity(
                    crate::server::utils::dvec3::DVec3::new(x as f64 + 0.5, y as f64 - crate::dungeon::room::fallingblocks::FALLING_FLOOR_ENTITY_OFFSET, z as f64 + 0.5),
                    {
                        let mut metadata = crate::server::entity::entity_metadata::EntityMetadata::new(crate::server::entity::entity_metadata::EntityVariant::Bat { hanging: false });
                        metadata.is_invisible = true;
                        metadata
                    },
                    crate::dungeon::room::fallingblocks::FallingFloorEntityImpl::new(current_block, 5.0, 20),
                );
            }
            
            // Remove the pattern from the room so it can't be triggered again
            let _ = self.fallingblock_patterns.remove(pattern_index);
        }
    }

    /// Process scheduled falling block removals (now handled by entities)
    pub fn process_scheduled_falling_removals(&mut self, _world: &mut World) {
        // This method is now empty since falling blocks are handled by entities
        // The scheduled_falling_removals field is kept for compatibility but not used
    }

    pub fn get_1x1_shape_and_type(segments: &[RoomSegment], dungeon_doors: &[Door]) -> (RoomShape, Direction) {
        let center_x = segments[0].x as i32 * 32 + 15 + DUNGEON_ORIGIN.0;
        let center_z = segments[0].z as i32 * 32 + 15 + DUNGEON_ORIGIN.1;

        // Actual doors found in the world
        let doors_opt = [
            (center_x, center_z - 16),
            (center_x + 16, center_z),
            (center_x, center_z + 16),
            (center_x - 16, center_z)
        ].iter().map(|pos| {
            dungeon_doors.iter()
                .find(|door| door.x == pos.0 && door.z == pos.1)
                .is_some()
        }).collect::<Vec<bool>>();

        let mut num: u8 = 0;
        for i in 0..4 {
            num <<= 1;
            num |= doors_opt[i] as u8;
        }

        // println!("{:04b} {:?}", num, doors_opt);

        match num {
            // Doors on all sides, never changes
            0b1111 => (RoomShape::OneByOneCross, Direction::North),
            // Dead end 1x1
            0b1000 => (RoomShape::OneByOneEnd, Direction::North),
            0b0100 => (RoomShape::OneByOneEnd, Direction::East),
            0b0010 => (RoomShape::OneByOneEnd, Direction::South),
            0b0001 => (RoomShape::OneByOneEnd, Direction::West),
            // Opposite doors
            0b0101 => (RoomShape::OneByOneStraight, Direction::North),
            0b1010 => (RoomShape::OneByOneStraight, Direction::East),
            // L bend
            0b0011 => (RoomShape::OneByOneBend, Direction::North),
            0b1001 => (RoomShape::OneByOneBend, Direction::East),
            0b1100 => (RoomShape::OneByOneBend, Direction::South),
            0b0110 => (RoomShape::OneByOneBend, Direction::West),
            // Triple door
            0b1011 => (RoomShape::OneByOneTriple, Direction::North),
            0b1101 => (RoomShape::OneByOneTriple, Direction::East),
            0b1110 => (RoomShape::OneByOneTriple, Direction::South),
            0b0111 => (RoomShape::OneByOneTriple, Direction::West),
            
            _ => (RoomShape::OneByOne, Direction::North),
        }
    }

    pub fn get_rotation_from_segments(segments: &[RoomSegment], dungeon_doors: &[Door]) -> Direction {
        let unique_x = segments.iter()
            .map(|segment| segment.x)
            .collect::<HashSet<usize>>();
        let unique_z = segments.iter()
            .map(|segment| segment.z)
            .collect::<HashSet<usize>>();

        let not_long = unique_x.len() > 1 && unique_z.len() > 1;

        match segments.len() {
            1 => {
                let (_, direction) = Room::get_1x1_shape_and_type(segments, dungeon_doors);
                direction
            },
            2 => match unique_z.len() == 1 {
                true => Direction::North,
                false => Direction::East,
            },
            3 => {  
                // L room
                if not_long {
                    let corner_value = segments.iter().find(|x| {
                        segments.iter().all(|y| {
                            x.x.abs_diff(y.x) + x.z.abs_diff(y.z) <= 1
                        })
                    }).expect("Invalid L room: Segments:");

                    let min_x = segments.iter().min_by(|a, b| a.x.cmp(&b.x)).unwrap().x;
                    let min_z = segments.iter().min_by(|a, b| a.z.cmp(&b.z)).unwrap().z;
                    let max_x = segments.iter().max_by(|a, b| a.x.cmp(&b.x)).unwrap().x;
                    let max_z = segments.iter().max_by(|a, b| a.z.cmp(&b.z)).unwrap().z;

                    if corner_value.x == min_x && corner_value.z == min_z {
                        return Direction::East
                    }
                    if corner_value.x == max_x && corner_value.z == min_z {
                        return Direction::South
                    }
                    if corner_value.x == max_x && corner_value.z == max_z {
                        return Direction::West
                    }
                    return Direction::North
                }

                match unique_z.len() == 1 {
                    true => Direction::North,
                    false => Direction::East,
                }
            },
            4 => {
                if unique_x.len() == 2 && unique_z.len() == 2 {
                    return Direction::North
                }

                match unique_z.len() == 1 {
                    true => Direction::North,
                    false => Direction::East,
                }
            },
            _ => unreachable!(),
        }
    }

    fn load_default(&self, world: &mut World) {
        for segment in self.segments.iter() {
            
            // Temporary for room colors, will be changed later on to paste saved room block states
            let block = match self.room_data.room_type {
                RoomType::Normal => Blocks::Stone { variant: 0 },
                RoomType::Blood => Blocks::Stone { variant: 0 },
                RoomType::Entrance => Blocks::Stone { variant: 0 },
                RoomType::Fairy => Blocks::Stone { variant: 0 },
                RoomType::Trap => Blocks::Stone { variant: 0 },
                RoomType::Yellow => Blocks::Stone { variant: 0 },
                RoomType::Puzzle => Blocks::Stone { variant: 0 },
                RoomType::Rare => Blocks::Stone { variant: 0 },
                RoomType::Boss => Blocks::Stone { variant: 0 },
            };

            world.fill_blocks(
                block,
                BlockPos {
                    x: segment.x as i32 * 32 + DUNGEON_ORIGIN.0,
                    y: self.room_data.bottom,
                    z: segment.z as i32 * 32 + DUNGEON_ORIGIN.1,
                },
                BlockPos {
                    x: segment.x as i32 * 32 + DUNGEON_ORIGIN.0 + 30,
                    y: self.room_data.bottom,
                    z: segment.z as i32 * 32 + DUNGEON_ORIGIN.1 + 30,
                }
            );

            // Merge to the side
            // if self.segments.contains(&(x+1, *z)) {
            //     world.fill_blocks(
            //         block,
            //         BlockPos {
            //             x: *x as i32 * 32 + 31 + DUNGEON_ORIGIN.0,
            //             y: self.room_data.bottom,
            //             z: *z as i32 * 32 + DUNGEON_ORIGIN.1,
            //         },
            //         BlockPos {
            //             x: *x as i32 * 32 + 31 + DUNGEON_ORIGIN.0,
            //             y: self.room_data.bottom,
            //             z: *z as i32 * 32 + DUNGEON_ORIGIN.1 + 30,
            //         }
            //     );
            // }
            // 
            // // // Merge below
            // if self.segments.contains(&(*x, z+1)) {
            //     world.fill_blocks(
            //         block,
            //         BlockPos {
            //             x: *x as i32 * 32 + DUNGEON_ORIGIN.0,
            //             y: self.room_data.bottom,
            //             z: *z as i32 * 32 + 31 + DUNGEON_ORIGIN.1,
            //         },
            //         BlockPos {
            //             x: * x as i32 * 32 + DUNGEON_ORIGIN.0 + 30,
            //             y: self.room_data.bottom,
            //             z: *z as i32 * 32 + 31 + DUNGEON_ORIGIN.1 + 30,
            //         }
            //     );
            // }
        }
    }

    /// Register levers for this room as interactable blocks
    pub fn register_levers(&self, world: &mut World) {
        for lever_data in &self.lever_data {
            let lever_pos = BlockPos {
                x: lever_data.lever[0],
                y: lever_data.lever[1],
                z: lever_data.lever[2],
            };
            
            // Register the lever as an interactable block
            world.interactable_blocks.insert(lever_pos, crate::server::block::block_interact_action::BlockInteractAction::Lever);
        }
    }

    pub fn load_into_world(&self, world: &mut World) {
        if self.room_data.block_data.is_empty() {
            self.load_default(world);
            return;
        }

        let corner = self.get_corner_pos();
        
        // Register levers for this room
        self.register_levers(world);

        for (i, block) in self.room_data.block_data.iter().enumerate() {
            if *block == Blocks::Air {
                continue;
            }
            // not sure if editing room data might ruin something,
            // so to be safe im just cloning it
            let mut block = block.clone();
            block.rotate(self.rotation);

            let ind = i as i32;

            let x = ind % self.room_data.width;
            let z = (ind / self.room_data.width) % self.room_data.length;
            let y = self.room_data.bottom + ind / (self.room_data.width * self.room_data.length);

            let bp = BlockPos { x, y, z }.rotate(self.rotation);

            world.set_block_at(block, corner.x + bp.x, y, corner.z + bp.z);
        }
    }

    // pub fn get_world_pos(&self, position: DVec3) -> DVec3 {
    //     let corner = self.get_corner_pos();
    //     position.clone()
    //         .rotate(self.rotation)
    //         .add_x(corner.x as f64)
    //         .add_z(corner.z as f64)
    // }

    pub fn get_world_block_pos(&self, room_pos: &BlockPos) -> BlockPos {
        let corner = self.get_corner_pos();

        room_pos.clone()
            .rotate(self.rotation)
            .add_x(corner.x)
            .add_z(corner.z)
    }
}


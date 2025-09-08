use crate::dungeon::crushers::Crusher;
use crate::dungeon::door::Door;
use crate::dungeon::dungeon::DUNGEON_ORIGIN;
use crate::dungeon::room::room_data::{RoomData, RoomShape, RoomType};
use crate::dungeon::room::crypts::{get_room_crypts, rotate_block_pos};
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

#[derive(Debug)]
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

        Room {
            segments,
            room_data,
            rotation,
            tick_amount: 0,
            crushers,
            crypt_patterns,
            crypts_checked: false,
            crypts_detected_count: 0,
            entered: false,
        }
    }

    pub fn get_corner_pos(&self) -> BlockPos {
        Room::get_corner_pos_from(&self.segments, &self.rotation, &self.room_data)
    }

    pub fn get_corner_pos_from(segments: &[RoomSegment], rotation: &Direction, room_data: &RoomData) -> BlockPos {
        let min_x = segments.iter().min_by(|a, b| a.x.cmp(&b.x)).unwrap().x;
        let min_z = segments.iter().min_by(|a, b| a.z.cmp(&b.z)).unwrap().z;

        let x = min_x as i32 * 32 + DUNGEON_ORIGIN.0;
        let y = 68;
        let z = min_z as i32 * 32 + DUNGEON_ORIGIN.1;
        
        match rotation {
            Direction::North => BlockPos { x, y, z },
            Direction::East => BlockPos { x: x + room_data.length - 1, y, z },
            Direction::South => BlockPos { x: x + room_data.length - 1, y, z: z + room_data.width - 1 },
            Direction::West => BlockPos { x: x, y, z: z + room_data.width - 1 },
            _ => unreachable!(),
        }
    }

    pub fn tick(&mut self) {
        self.tick_amount += 1;
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

    pub fn load_into_world(&self, world: &mut World) {
        if self.room_data.block_data.is_empty() {
            self.load_default(world);
            return;
        }

        let corner = self.get_corner_pos();

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


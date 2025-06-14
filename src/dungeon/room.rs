use std::collections::HashSet;

use serde_json::json;

use crate::{dungeon::{door::Door, room_data::{RoomData, RoomShape, RoomType}, DUNGEON_ORIGIN}, server::{block::{block_pos::BlockPos, blocks::Blocks}, utils::direction::Direction, world::World}};

pub struct Room {
    pub segments: Vec<(usize, usize)>,
    pub room_data: RoomData,
    pub rotation: Direction,

    pub tick_amount: u32,
}

impl Room {

    pub fn new(
        mut segments: Vec<(usize, usize)>,
        dungeon_doors: &Vec<Door>,
        room_data: RoomData
    ) -> Room {
        // Sort room segments by y and then x
        segments.sort_by(|a, b| a.1.cmp(&b.1));
        segments.sort_by(|a, b| a.0.cmp(&b.0));

        let rotation = Room::get_rotation_from_segments(&segments, dungeon_doors);
        
        Room {
            segments,
            room_data,
            rotation,
            tick_amount: 0,
        }
    }

    pub fn get_corner_pos(&self) -> BlockPos {
        let first_segment = self.segments[0];

        let x = first_segment.0 as i32 * 32 + DUNGEON_ORIGIN.0;
        let y = 68;
        let z = first_segment.1 as i32 * 32 + DUNGEON_ORIGIN.1;

        // BlockPos { x, y, z }
        match self.rotation {
            Direction::North => BlockPos { x, y, z },
            Direction::East => BlockPos { x: x + self.room_data.length - 1, y, z },
            Direction::South => BlockPos { x: x + self.room_data.length - 1, y, z: z + self.room_data.width - 1 },
            Direction::West => BlockPos { x, y, z: z + self.room_data.width - 1 }
        }
    }

    pub fn tick(&mut self) {
        self.tick_amount += 1;
    }

    pub fn get_rotation_from_segments(segments: &Vec<(usize, usize)>, dungeon_doors: &Vec<Door>) -> Direction {
        let unique_x = segments.iter()
            .map(|x| x.0)
            .collect::<HashSet<usize>>();

        let unique_z = segments.iter()
            .map(|x| x.1)
            .collect::<HashSet<usize>>();

        let not_long = unique_x.len() > 1 && unique_z.len() > 1;

        match segments.len() {
            1 => {
                // Check door locations
                Direction::North
            },
            2 => match unique_z.len() == 1 {
                true => Direction::North,
                false => Direction::East,
            },
            3 => {  
                // L room
                if not_long {
                    // TODO
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
        for (x, z) in self.segments.iter() {
            
            // Temporary for room colors, will be changed later on to paste saved room block states
            let block = match self.room_data.room_type {
                RoomType::Normal => Blocks::BrownWool,
                RoomType::Blood => Blocks::RedWool,
                RoomType::Entrance => Blocks::GreenWool,
                RoomType::Fairy => Blocks::PinkWool,
                RoomType::Trap => Blocks::OrangeWool,
                RoomType::Yellow => Blocks::YellowWool,
                RoomType::Puzzle => Blocks::PurpleWool,
            };

            world.fill_blocks(
                block,
                (
                    *x as i32 * 32 + DUNGEON_ORIGIN.0,
                    self.room_data.bottom,
                    *z as i32 * 32 + DUNGEON_ORIGIN.1,
                ),
                (
                    *x as i32 * 32 + DUNGEON_ORIGIN.0 + 30,
                    self.room_data.bottom,
                    *z as i32 * 32 + DUNGEON_ORIGIN.1 + 30,
                )
            );

            // Merge to the side
            if self.segments.contains(&(x+1, *z)) {
                world.fill_blocks(
                    block,
                    (
                        *x as i32 * 32 + 31 + DUNGEON_ORIGIN.0,
                        self.room_data.bottom,
                        *z as i32 * 32 + DUNGEON_ORIGIN.1,
                    ),
                    (
                        *x as i32 * 32 + 31 + DUNGEON_ORIGIN.0,
                        self.room_data.bottom,
                        *z as i32 * 32 + DUNGEON_ORIGIN.1 + 30,
                    )
                );
            }
            
            // // Merge below
            if self.segments.contains(&(*x, z+1)) {
                world.fill_blocks(
                    block,
                    (
                        *x as i32 * 32 + DUNGEON_ORIGIN.0,
                        self.room_data.bottom,
                        *z as i32 * 32 + 31 + DUNGEON_ORIGIN.1,
                    ),
                    (
                        *x as i32 * 32 + DUNGEON_ORIGIN.0 + 30,
                        self.room_data.bottom,
                        *z as i32 * 32 + 31 + DUNGEON_ORIGIN.1 + 30,
                    )
                );
            }
        }
    }

    pub fn load_into_world(&self, world: &mut World) {
        if self.room_data.block_data.len() == 0 {
            self.load_default(world);
            return;
        }
        // self.load_default(world);
        // return;

        let corner = self.get_corner_pos();

        for (i, block) in self.room_data.block_data.iter().enumerate() {
            if *block == Blocks::Air {
                continue;
            }

            let ind = i as i32;

            let x = ind % self.room_data.width;
            let z = (ind / self.room_data.width) % self.room_data.length;
            let y = self.room_data.bottom + ind / (self.room_data.width * self.room_data.length);

            let bp = BlockPos { x, y, z }.rotate(&self.rotation);

            world.set_block_at(*block, corner.x + bp.x, y, corner.z + bp.z);
        }

    }
}


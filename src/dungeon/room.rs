use serde_json::json;

use crate::{dungeon::{room_data::{RoomData, RoomType}, DUNGEON_ORIGIN}, server::{block::{block_pos::BlockPos, blocks::Blocks}, world::World}};

pub struct Room {
    pub segments: Vec<(usize, usize)>,
    pub room_data: RoomData,

    pub tick_amount: u32,
}

impl Room {

    pub fn new(mut segments:  Vec<(usize, usize)>, room_data: RoomData) -> Room {
        // Sort room segments by y and then x
        segments.sort_by(|a, b| a.1.cmp(&b.1));
        segments.sort_by(|a, b| a.0.cmp(&b.0));

        Room {
            segments,
            room_data,
            tick_amount: 0,
        }
    }

    pub fn get_corner_pos(&self) -> BlockPos {
        let first_segment = self.segments[0];

        // TODO: Room Rotation

        BlockPos {
            x: first_segment.0 as i32 * 32 + DUNGEON_ORIGIN.0,
            y: 68,
            z: first_segment.1 as i32 * 32 + DUNGEON_ORIGIN.1,
        }
    }

    pub fn tick(&mut self) {
        self.tick_amount += 1;
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

        for (i, block) in self.room_data.block_data.iter().enumerate() {
            let ind = i as i32;

            let x = ind % self.room_data.width;
            let z = (ind / self.room_data.width) % self.room_data.length;
            let y = self.room_data.bottom + ind / (self.room_data.width * self.room_data.length);

            world.set_block_at(*block, x, y, z);
        }

    }
}


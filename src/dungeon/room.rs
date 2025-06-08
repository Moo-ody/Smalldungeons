use crate::{dungeon::DUNGEON_ORIGIN, server::block::block_pos::BlockPos};



pub struct Room {
    pub segments: Vec<(usize, usize)>,
    pub room_type: RoomType,

    pub tick_amount: u32,
}

pub enum RoomShape {
    OneByOne,
    OneByTwo,
    OneByThree,
    OneByFour,
    TwoByTwo,
    L,
}

pub enum RoomType {
    Normal,
    Puzzle,
    Trap,
    Fairy,
    Entrance,
    Blood,
    Yellow,
}

impl Room {

    pub fn new(mut segments:  Vec<(usize, usize)>, room_type: RoomType) -> Room {
        // Sort room segments by y and then x
        segments.sort_by(|a, b| a.1.cmp(&b.1));
        segments.sort_by(|a, b| a.0.cmp(&b.0));

        Room {
            segments,
            room_type,
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
}
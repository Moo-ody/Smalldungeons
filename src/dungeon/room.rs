use crate::dungeon::DUNGEON_ORIGIN;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::utils::direction::Direction;
use crate::server::world::World;

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
    // pub fn get_corner_pos(&self) -> BlockPos {
    //     BlockPos {
    //         x: self.room_x as i32 * 32 + DUNGEON_ORIGIN.0,
    //         y: 68,
    //         z: self.room_z as i32 * 32 + DUNGEON_ORIGIN.1,
    //     }
    // }

    // pub fn get_segment_locations(&self) -> Vec<(usize, usize)> {
    //     match self.shape {
    //         RoomShape::OneByOne => vec![(self.room_x, self.room_z)],
    //         RoomShape::OneByTwo => vec![
    //             (self.room_x, self.room_z),
    //             (self.room_x + 1, self.room_z),
    //         ],
    //         RoomShape::OneByThree => vec![
    //             (self.room_x, self.room_z),
    //             (self.room_x + 1, self.room_z),
    //             (self.room_x + 1, self.room_z),
    //         ],
    //         RoomShape::OneByFour => vec![
    //             (self.room_x, self.room_z),
    //             (self.room_x + 1, self.room_z),
    //             (self.room_x + 2, self.room_z),
    //             (self.room_x + 3, self.room_z),
    //         ],
    //         RoomShape::TwoByTwo => vec![
    //             (self.room_x, self.room_z),
    //             (self.room_x + 1, self.room_z),
    //             (self.room_x, self.room_z + 1),
    //             (self.room_x + 1, self.room_z + 1),
    //         ],
    //         RoomShape::L => vec![
    //             (self.room_x, self.room_z),
    //             (self.room_x, self.room_z + 1),
    //             (self.room_x + 1, self.room_z + 1),
    //         ]
    //     }
    // }


    pub fn tick(&mut self) {
        self.tick_amount += 1;
    }
}
use crate::dungeon::room::room::Room;
use crate::dungeon::room::room_data::{RoomData, RoomType::*};
use crate::server::block::block_parameter::Axis;

const RED: u8 = 4 * 4 + 2;
const GREEN: u8 = 7 * 4 + 2;
const GRAY: u8 = 11 * 4 + 3;
const WHITE: u8 = 14 * 4 + 2;
const ORANGE: u8 = 15 * 4 + 2; 
const PURPLE: u8 = 16 * 4 + 2;
const PINK: u8 = 20 * 4 + 2;
const BLACK: u8 = 29 * 4;
const YELLOW: u8 = 30 * 4 + 2;
const BROWN: u8 = 63;

pub struct DungeonMap {
    pub map_data: [u8; 128 * 128]
}

// room is 16x16 px
// gap is 4 px
// door is 5x4 px
impl DungeonMap {

    pub fn new() -> Self {
        Self {
            map_data: [0; 128 * 128],
        }
    }

    pub fn fill_px(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: u8,
    ) {
        debug_assert!(x + width <= 128);
        debug_assert!(y + height <= 128);

        for x in x..x+width {
            for y in y..y+height {
                self.map_data[y * 128 + x] = color
            }
        }
    }

    pub fn draw_room(&mut self, room: &Room) {
        // this function should not be called on a room that hasn't been entered
        debug_assert!(room.entered);

        let color = get_room_color(&room.room_data);

        for segment in room.segments.iter() {
            let x = segment.x * 20;
            let mut y = segment.z * 20;
            let mut width = 16;
            let mut height = 16;

            if room.segments.iter().find(|seg| seg.x == segment.x + 1 && seg.z == segment.z).is_some() {
                width += 4;
            }
            if segment.z != 0 && room.segments.iter().find(|seg| seg.x == segment.x && seg.z == segment.z - 1).is_some() {
                y -= 4;
                height += 4;
            }
            self.fill_px(x, y, width, height, color);


            for (index, neighbour) in segment.neighbours.iter().enumerate() {
                if neighbour.is_none() {
                    continue;
                }
                let (neighbour, door) = {
                    let neighbour = neighbour.as_ref().unwrap();
                    (neighbour.room.borrow(), neighbour.door.borrow())
                };

                let mut x = segment.x * 20 + 6;
                let mut y = segment.z * 20 + 6;

                match index {
                    0 => y -= 10,
                    1 => x += 10,
                    2 => y += 10,
                    3 => x -= 10,
                    _ => unreachable!()
                }

                let (width, height) = match door.direction {
                    Axis::X => (4, 5),
                    Axis::Z => (5, 4),
                    _ => unreachable!()
                };

                if neighbour.entered {
                    let color = get_door_color(room, &*neighbour);
                    self.fill_px(x, y, width, height, color);
                } else {
                    self.fill_px(x, y, width, height, GRAY)
                }
            }
        }
    }
}

fn get_room_color(room_data: &RoomData) -> u8 {
    match room_data.room_type {
        Normal | Rare => BROWN,
        Puzzle => PURPLE,
        Trap => ORANGE,
        Fairy => PINK,
        Entrance => GREEN,
        Blood => RED,
        Yellow =>  YELLOW,
    }
}

fn get_door_color(room: &Room, neighbour: &Room) -> u8 {
    match room.room_data.room_type {
        Puzzle | Trap | Blood | Yellow | Fairy => {
            return get_room_color(&room.room_data)
        }
        _ => {}
    };
    match neighbour.room_data.room_type {
        Puzzle | Trap | Blood | Yellow | Fairy => {
            return get_room_color(&neighbour.room_data)
        }
        _ => {}
    };
    BROWN
}
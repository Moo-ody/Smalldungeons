use crate::server::block::blocks::Blocks;
use crate::server::world::World;

pub struct Room {
    pub room_x: i32,
    pub room_z: i32,

    pub tick_amount: u32,

    pub room_type: RoomType,
}

pub enum RoomType {
    Shape1x1,
    Shape2x1,
    Shape2x2,
}

impl Room {


    pub fn load_room(&self, world: &mut World) {
        match self.room_type {
            RoomType::Shape1x1 => {

                let start_x = self.room_x * 33;
                let start_z = self.room_z * 33;

                for x in start_x..start_x + 32 {
                    for z in start_z..start_z + 32 {
                        world.set_block_at(Blocks::Stone, x, 0, z);
                    }
                }
            }
            RoomType::Shape2x1 => {

                let start_x = self.room_x * 33;
                let start_z = self.room_z * 33;

                for x in start_x..start_x + 32 {
                    for z in start_z..start_z + 63 {
                        world.set_block_at(Blocks::Stone, x, 0, z);
                    }
                }
            }
            RoomType::Shape2x2 => {
                let start_x = self.room_x * 33;
                let start_z = self.room_z * 33;

                for x in start_x..start_x + 63 {
                    for z in start_z..start_z + 63 {
                        world.set_block_at(Blocks::Stone, x, 0, z);
                    }
                }
            }
        }
    }


    pub fn tick(&mut self) {
        self.tick_amount += 1;
    }
}
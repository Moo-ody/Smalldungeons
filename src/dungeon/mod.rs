use crate::dungeon::room::{Room, RoomType};
use crate::server::player::Player;

pub mod room;
pub mod crushers;

// contains a vec of rooms,
// also contains a grid, containing indices pointing towards the rooms,
//
// contains a vec of doors (for generation)
pub struct Dungeon {
    pub rooms: Vec<Room>,
    // maybe don't use 36 in case of a smaller map ?
    pub index_grid: [u8; 36],
}

impl Dungeon {

    pub fn with_rooms(rooms: Vec<Room>) -> Dungeon {
        let mut index_grid = [0; 36];

        // populate index grid
        for (index, room) in rooms.iter().enumerate() {
            match room.room_type {
                RoomType::Shape1x1 => {
                    let pos = (room.room_x + (room.room_z * 6)) as usize;
                    index_grid[pos] = index as u8 + 1;
                }
                RoomType::Shape2x1 => {
                    let pos = (room.room_x + (room.room_z * 6)) as usize;
                    index_grid[pos] = index as u8 + 1;
                    let pos = (room.room_x + ((room.room_z + 1) * 6)) as usize;
                    index_grid[pos] = index as u8 + 1;
                }

                RoomType::Shape2x2 => {
                    let pos = (room.room_x + (room.room_z * 6)) as usize;
                    index_grid[pos] = index as u8 + 1;
                    let pos = (room.room_x + ((room.room_z + 1) * 6)) as usize;
                    index_grid[pos] = index as u8 + 1;
                    let pos = (room.room_x + 1 + (room.room_z * 6)) as usize;
                    index_grid[pos] = index as u8 + 1;
                    let pos = (room.room_x + 1 + ((room.room_z + 1) * 6)) as usize;
                    index_grid[pos] = index as u8 + 1;
                }
            }
        }

        println!("grid {:?}", &index_grid);

        Dungeon {
            rooms,
            index_grid,
        }
    }

    pub fn get_room(&self, player: &Player) -> Option<&Room> {
        let server = player.server_mut();
        let entity = player.get_entity(&server.world).unwrap();
        let (x, _, z) = (
            entity.pos.x as i32 / 32,
            entity.pos.y as i32 / 32,
            entity.pos.z as i32 / 32
        );
        if let Some(room_index) = self.index_grid.get((x + (z * 6)) as usize) {
            if *room_index == 0 {
                return None
            }
        }
        None
        //
        // if  {  }
        //
        // println!(
        //     "entity pos: x: {}, y: {} z: {}",
        //     entity.pos.x as i32 / 32,
        //     entity.pos.y as i32 / 32,
        //     entity.pos.z as i32 / 32
        // )
    }

}
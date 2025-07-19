use crate::dungeon::door::{Door, DoorType};
use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::room::Room;
use crate::dungeon::room_data::{get_random_data_with_type, RoomData, RoomShape, RoomType};
use crate::net::packets::packet::SendPacket;
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_parameter::Axis;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::EntityId;
use crate::server::player::player::Player;
use crate::server::server::Server;
use crate::server::world;
use anyhow::bail;
use rand::seq::IndexedRandom;
use std::collections::HashMap;

pub mod room;
pub mod door;
pub mod crushers;
pub mod room_data;
pub mod dungeon_state;

// The top leftmost corner of the dungeon
const DUNGEON_ORIGIN: (i32, i32) = (0, 0);

// The positions of the doors in the world
const DOOR_POSITIONS: [(i32, i32); 60] = [(DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 15), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 15), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 15), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 15), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 15), (DUNGEON_ORIGIN.0 + 15, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 47, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 79, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 111, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 143, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 175, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 47), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 47), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 47), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 47), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 47), (DUNGEON_ORIGIN.0 + 15, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 47, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 79, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 111, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 143, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 175, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 79), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 79), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 79), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 79), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 79), (DUNGEON_ORIGIN.0 + 15, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 47, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 79, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 111, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 143, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 175, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 111), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 111), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 111), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 111), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 111), (DUNGEON_ORIGIN.0 + 15, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 47, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 79, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 111, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 143, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 175, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 143), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 143), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 143), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 143), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 143), (DUNGEON_ORIGIN.0 + 15, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 47, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 79, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 111, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 143, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 175, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 175), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 175), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 175), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 175), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 175)];

// contains a vec of rooms,
// also contains a grid, containing indices pointing towards the rooms,
//
// contains a vec of doors (for generation)
pub struct Dungeon {
    pub server: *mut Server,
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    // The numer in this grid will be 0 if there is no room here, or contain
    // The index - 1 of the room here from &rooms
    pub index_grid: [usize; 36],
    pub state: DungeonState,
    pub test: Vec<OpenDoorTask>
}

// Better name?
// maybe have a global tick queue 
// currently, what it does is: when ticks left reaches 0 it clears the blocks the door has
pub struct OpenDoorTask {
    pub ticks_left: usize,
    pub door_index: usize,
    pub door_entity_ids: Vec<EntityId>,
}

impl Dungeon {
    pub fn with_rooms_and_doors(rooms: Vec<Room>, doors: Vec<Door>) -> anyhow::Result<Dungeon> {
        let mut index_grid = [0; 36];

        // populate index grid
        for (room_index, room) in rooms.iter().enumerate() {
            for (x, z) in room.segments.iter() {
                let segment_index = x + z * 6;

                if segment_index > index_grid.len() - 1 {
                    bail!("Segment index for {},{} out of bounds: {}", x, z, segment_index);
                }

                if index_grid[segment_index] != 0 {
                    bail!("Segment at {},{} is already occupied by {}!", x, z, index_grid[segment_index]);
                }

                index_grid[segment_index] = room_index + 1;
            }
        }

        // println!("grid {:?}", &index_grid);

        Ok(Dungeon {
            server: std::ptr::null_mut(),
            rooms,
            doors,
            index_grid,
            state: DungeonState::NotReady,
            test: Vec::new(),
        })
    }

    pub fn server_mut<'a>(&self) -> &'a mut Server {
        unsafe { self.server.as_mut().expect("server is null") }
    }

    // Layout String:
    // 36 x room ids, two digits long each. 00 = no room, 01 -> 06 are special rooms like spawn, puzzles etc
    // 07 -> ... are normal rooms, with unique ids to differentiate them and preserve layout
    // Doors are 60x single digit numbers in the order left -> right top -> down for every spot they can possibly spawn
    pub fn from_string(layout_str: &str, room_data_storage: &HashMap<usize, RoomData>) -> anyhow::Result<Dungeon> {
        let mut rooms: Vec<Room> = Vec::new();
        // For normal rooms which can be larger than 1x1, store their segments and make the whole room in one go later
        let mut room_id_map: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();

        let mut doors: Vec<Door> = Vec::new();

        for i in 0..60usize {
            let type_str = layout_str.get(i + 72..i+73).unwrap();
            let (x, z) = DOOR_POSITIONS[i];

            let door_type = match type_str {
                "0" => Some(DoorType::NORMAL),
                "1" => Some(DoorType::WITHER),
                "2" => Some(DoorType::BLOOD),
                "3" => Some(DoorType::ENTRANCE),
                _ => None,
            };

            if door_type.is_some() {
                // println!("{}", (x - DUNGEON_ORIGIN.0) / 16);
                let direction = match ((x - DUNGEON_ORIGIN.0) / 16) % 2 {
                    0 => Axis::Z,
                    1 => Axis::X,
                    _ => unreachable!(),
                };

                doors.push(Door {
                    id: doors.len(),
                    x,
                    z,
                    direction,
                    door_type: door_type.unwrap()
                })
            }
        }

        for i in 0..36 {
            let substr = layout_str.get(i*2..i*2+2);
            let x = i % 6;
            let z = i / 6;

            // Shouldn't happen if data is not corrupted
            if substr.is_none() {
                panic!("Failed to parse dungeon string: too small.")
            }

            let id = substr.unwrap().parse::<usize>()?;

            // No room here
            if id == 0 {
                continue;
            }

            // Special rooms
            if id <= 6 {
                let room_type = match id {
                    1 => RoomType::Entrance,
                    2 => RoomType::Fairy,
                    3 => RoomType::Blood,
                    4 => RoomType::Puzzle,
                    5 => RoomType::Trap,
                    6 => RoomType::Yellow,
                    _ => unreachable!()
                };

                // Fairy can have a varying number of doors, all other special rooms are fixed to just one.
                let shape = match room_type {
                    RoomType::Fairy => RoomShape::OneByOne,
                    _ => RoomShape::OneByOneEnd,
                };

                let mut room_data = get_random_data_with_type(
                    room_type,
                    shape,
                    room_data_storage,
                    &rooms
                );

                room_data.room_type = room_type;

                rooms.push(Room::new(
                    vec![(x, z)],
                    &doors,
                    room_data
                ));

                continue
            }

            // Normal rooms, add segments to this specific room id
            let entry = room_id_map.entry(id).or_default();
            entry.push((x, z));
        }

        // Make the normal rooms
        for (_, segments) in room_id_map {
            let shape = RoomShape::from_segments(&segments, &doors);

            rooms.push(Room::new(
                segments,
                &doors,
                get_random_data_with_type(
                    RoomType::Normal,
                    shape,
                    room_data_storage,
                    &rooms
                )
            ));
        }

        Dungeon::with_rooms_and_doors(rooms, doors)
    }

    pub fn get_room_at(&mut self, x: i32, z: i32) -> Option<&mut Room> {
        if x < DUNGEON_ORIGIN.0 || z < DUNGEON_ORIGIN.1 {
            return None;
        }

        let grid_x = ((x - DUNGEON_ORIGIN.0) / 32) as usize;
        let grid_z = ((z - DUNGEON_ORIGIN.1) / 32) as usize;

        // The returned number is 0 if no room here, or will return the index + 1 of the room in the rooms vec
        let entry = self.index_grid.get(grid_x + (grid_z * 6));

        if entry.is_none_or(|index| *index == 0) {
            return None;
        }

        self.rooms.get_mut(*entry.unwrap() - 1)
    }

    pub fn get_player_room(&mut self, player: &Player) -> Option<&mut Room> {
        self.get_room_at(
            player.position.x as i32,
            player.position.z as i32
        )
    }
    
    pub fn start_dungeon(&mut self) {
        let world = &mut self.server_mut().world;
        for (index, door) in self.doors.iter_mut().enumerate() {
            if door.door_type == DoorType::ENTRANCE {
                door.open_door(world);
                continue;
            }
            
            if door.door_type == DoorType::NORMAL { 
                continue;
            }
            
            world::iterate_blocks(
                BlockPos { x: door.x - 1, y: 69, z: door.z - 1 },
                BlockPos { x: door.x + 1, y: 72, z: door.z + 1 },
                |x, y, z| {
                    let action = match door.door_type {
                        DoorType::WITHER => BlockInteractAction::WitherDoor { door_index: index },
                        DoorType::BLOOD => BlockInteractAction::BloodDoor { door_index: index },
                        _ => unreachable!()
                    };
                    world.interactable_blocks.insert(BlockPos::new(x, y, z), action);
                }
            );
        }
        // probably mark room connected to entrance as entered 
    }

    // TODO: all dungeon-ticking happens from here,
    pub fn tick(&mut self) -> anyhow::Result<()> {
        let server = self.server_mut();
        
        match &mut self.state {
            DungeonState::NotReady | DungeonState::Finished => {}
            
            DungeonState::Starting { tick_countdown: tick } => {
                *tick -= 1;
                if *tick == 0 {
                    self.state = DungeonState::Started { current_ticks: 0 };
                    self.start_dungeon();
                } else if *tick % 20 == 0 {
                    
                    let seconds_remaining = *tick / 20;
                    let s = if seconds_remaining == 1 { "" } else { "s" };
                    let str = format!("§aStarting in {} second{}.", seconds_remaining, s);

                    for (_, player) in &server.world.players {
                        player.send_msg(&str)?;
                    }
                }
            }
            
            DungeonState::Started { current_ticks } => {
                *current_ticks += 1;
                
                for (_, player) in &server.world.players  {
                    if let Some(room) = server.dungeon.get_player_room(player) {
                        for crusher in room.crushers.iter_mut() {
                            crusher.tick(player);
                        }
                    }
                }
                
                self.test.retain_mut(move |test| {
                    test.ticks_left -= 1;
                    if test.ticks_left == 0 {
                        let door = &server.dungeon.doors[test.door_index];
                        server.world.fill_blocks(
                            Blocks::Air,
                            BlockPos { x: door.x - 1, y: 69, z: door.z - 1 },
                            BlockPos { x: door.x + 1, y: 72, z: door.z + 1 },
                        );
                        for entity_id in &test.door_entity_ids {
                            server.world.despawn_entity(*entity_id).unwrap()
                        }
                        false
                    } else {
                        true
                    }
                });
            }
        }
        Ok(())
    }
}
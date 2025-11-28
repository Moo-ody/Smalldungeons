use crate::dungeon::door::{Door, DoorType};
use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::map::DungeonMap;
use crate::dungeon::room::room::{Room, RoomNeighbour, RoomSegment};
use crate::dungeon::room::room_data::{get_random_data_with_type, RoomData, RoomShape, RoomType};
use crate::net::protocol::play::clientbound::Maps;
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_parameter::Axis;
use crate::server::block::block_position::BlockPos;
use crate::server::player::player::Player;
use crate::server::server::Server;
use crate::server::utils::dvec3::DVec3;
use crate::server::world;
use crate::server::utils::sounds::Sounds;
use crate::net::protocol::play::clientbound::SoundEffect;
use crate::utils::hasher::deterministic_hasher::DeterministicHashMap;
use anyhow::bail;
use std::collections::HashMap;
// use crate::server::block::block_interact_action::BlockInteractAction::*;

// The top leftmost corner of the dungeon
pub const DUNGEON_ORIGIN: (i32, i32) = (-200, -200);

// The positions of the doors in the world
pub const DOOR_POSITIONS: [(i32, i32); 60] = [(DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 15), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 15), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 15), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 15), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 15), (DUNGEON_ORIGIN.0 + 15, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 47, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 79, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 111, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 143, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 175, DUNGEON_ORIGIN.1 + 31), (DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 47), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 47), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 47), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 47), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 47), (DUNGEON_ORIGIN.0 + 15, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 47, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 79, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 111, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 143, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 175, DUNGEON_ORIGIN.1 + 63), (DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 79), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 79), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 79), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 79), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 79), (DUNGEON_ORIGIN.0 + 15, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 47, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 79, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 111, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 143, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 175, DUNGEON_ORIGIN.1 + 95), (DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 111), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 111), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 111), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 111), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 111), (DUNGEON_ORIGIN.0 + 15, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 47, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 79, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 111, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 143, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 175, DUNGEON_ORIGIN.1 + 127), (DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 143), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 143), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 143), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 143), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 143), (DUNGEON_ORIGIN.0 + 15, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 47, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 79, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 111, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 143, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 175, DUNGEON_ORIGIN.1 + 159), (DUNGEON_ORIGIN.0 + 31, DUNGEON_ORIGIN.1 + 175), (DUNGEON_ORIGIN.0 + 63, DUNGEON_ORIGIN.1 + 175), (DUNGEON_ORIGIN.0 + 95, DUNGEON_ORIGIN.1 + 175), (DUNGEON_ORIGIN.0 + 127, DUNGEON_ORIGIN.1 + 175), (DUNGEON_ORIGIN.0 + 159, DUNGEON_ORIGIN.1 + 175)];

/// State for a locked chest
#[derive(Debug, Clone)]
pub struct LockedChestState {
    pub locked: bool,
    pub lever_world_pos: BlockPos,
}

// contains a vec of rooms,
// also contains a grid, containing indices pointing towards the rooms,
//
// contains a vec of doors (for generation)
pub struct Dungeon {
    pub server: *mut Server,
    pub doors: Vec<Door>,
    pub rooms: Vec<Room>,

    pub room_grid: [Option<usize>; 36],
    pub state: DungeonState,
    pub map: DungeonMap,

    // Temporary per-player mapping of mushroom set index -> up destination (world BlockPos)
    pub temp_player_mushroom_up: HashMap<u32, Vec<BlockPos>>,
    
    // Locked chest system
    // Maps chest world position to its locked state and associated lever
    pub locked_chests: HashMap<BlockPos, LockedChestState>,
    // Maps lever world position to all chests it unlocks
    pub lever_to_chests: HashMap<BlockPos, Vec<BlockPos>>,
    
    // Boss room data
    // pub boss_room_corner: BlockPos,
    // pub boss_room_width: i32,
    // pub boss_room_length: i32,
    // pub boss_room_height: i32,
    
}

impl Dungeon {
    pub fn from_layout(doors: Vec<Door>, mut rooms: Vec<Room>) -> anyhow::Result<Dungeon> {
        let mut room_grid: [Option<usize>; 36] = [const { None }; 36];
        let mut grid_max_x = 0;
        let mut grid_max_y = 0;

        for (index, room) in rooms.iter().enumerate() {
            for segment in room.segments.iter() {
                let x = segment.x;
                let z = segment.z;
                let segment_index = (x + z * 6) as usize;
    
                if segment_index > room_grid.len() - 1 {
                    bail!("Segment index for {},{} out of bounds: {}", x, z, segment_index);
                }
                if room_grid[segment_index].is_some() {
                    bail!("Segment at {},{} is already occupied by {:?}!", x, z, room_grid[segment_index]);
                }
                room_grid[segment_index] = Some(index);
    
                if x > grid_max_x {
                    grid_max_x = x;
                }
                if z > grid_max_y {
                    grid_max_y = z;
                }
            }
        }
        
        for room in rooms.iter_mut() {
            let segments = &mut room.segments;
            
            let segment_positions = segments.iter()
                .map(|segment| (segment.x, segment.z))
                .collect::<Vec<(usize, usize)>>();
            
            for segment in room.segments.iter_mut() {
                let x = segment.x as isize;
                let z = segment.z as isize;
                let center_x = segment.x as i32 * 32 + 15 + DUNGEON_ORIGIN.0;
                let center_z = segment.z as i32 * 32 + 15 + DUNGEON_ORIGIN.1;
                
                let neighbour_options = [
                    (x, z - 1, center_x, center_z - 16),
                    (x + 1, z, center_x + 16, center_z),
                    (x, z + 1, center_x, center_z + 16),
                    (x - 1, z, center_x - 16, center_z),
                ];
                
                for (index, (nx, nz, door_x, door_z)) in neighbour_options.into_iter().enumerate() {
                    if nx < 0 || nz < 0 || segment_positions.iter().find(|(x, z)| *x as isize == nx && *z as isize == nz).is_some() {
                        continue;
                    }
                    
                    let door = doors.iter().enumerate().find(|(_, door)| {
                        door.x == door_x && door.z == door_z
                    });
                    
                    if let Some((door_index, _)) = door {
                        segment.neighbours[index] = Some(RoomNeighbour {
                            door_index: door_index,
                            room_index: room_grid[(nx + nz * 6) as usize].expect("Neighbor should be Some")
                        });
                    }
                }
            }
        }
        
        let map_offset_x = (128 - (grid_max_x + 1) * 20) / 2;
        let map_offset_y = (128 - (grid_max_y + 1) * 20) / 2;
        
        Ok(Dungeon {
            server: std::ptr::null_mut(),
            doors,
            rooms,
            room_grid: room_grid,
            state: DungeonState::NotReady,
            map: DungeonMap::new(map_offset_x, map_offset_y),
            temp_player_mushroom_up: HashMap::new(),
            locked_chests: HashMap::new(),
            lever_to_chests: HashMap::new(),
            // boss_room_corner: BlockPos { x: -8, y: 254, z: -8 },
            // boss_room_width: 0, // Will be set when boss room is loaded
            // boss_room_length: 0, // Will be set when boss room is loaded
            // boss_room_height: 30, // Default height
        })
    }


    pub fn from_str(layout_str: &str, room_data_storage: &DeterministicHashMap<usize, RoomData>) -> anyhow::Result<Dungeon> {
        let mut rooms: Vec<Room> = Vec::new();
        let mut doors: Vec<Door> = Vec::new();

        let mut room_id_map: DeterministicHashMap<usize, Vec<RoomSegment>> = DeterministicHashMap::default();

        for (index, (x, z)) in DOOR_POSITIONS.into_iter().enumerate() {
            let type_str = layout_str.get(index + 72..index+73).unwrap();

            let door_type = match type_str {
                "0" => Some(DoorType::NORMAL),
                "1" => Some(DoorType::WITHER),
                "2" => Some(DoorType::BLOOD),
                "3" => Some(DoorType::ENTRANCE),
                _ => None,
            };

            if let Some(door_type) = door_type {
                let direction = match ((x - DUNGEON_ORIGIN.0) / 16) % 2 {
                    0 => Axis::Z,
                    1 => Axis::X,
                    _ => unreachable!(),
                };

                let door = Door {
                    x,
                    z,
                    direction,
                    door_type
                };

                doors.push(door);
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
                    RoomType::Boss => RoomShape::FourByFour,
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
                    vec![RoomSegment { x, z, neighbours: [const { None }; 4] }],
                    &doors,
                    room_data
                ));

                continue
            }

            // Normal rooms, add segments to this specific room id
            let entry = room_id_map.entry(id).or_default();
            entry.push(RoomSegment { x, z, neighbours: [const { None }; 4] });
        }

        // Make the normal rooms
        rooms.reserve(room_id_map.len());
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

        Self::from_layout(doors, rooms)
    }

    // pub fn with_rooms_and_doors(rooms: Vec<Room>, doors: Vec<Door>) -> anyhow::Result<Dungeon> {

    //     let mut room_grid = [0; 36];
    //     let mut grid_max_x = 0;
    //     let mut grid_max_y = 0;

    //     let rooms = rooms.into_iter().map(|room| Rc::new(RefCell::new(room))).collect::<Vec<Rc<RefCell<Room>>>>();
    //     let doors = doors.into_iter().map(|door| Rc::new(RefCell::new(door))).collect::<Vec<Rc<RefCell<Door>>>>();

    //     // populate index grid
    //     for (room_index, room_rc) in rooms.iter().enumerate() {
    //         let room = room_rc.borrow();
    //         for segment in room.segments.iter() {
    //             let x = segment.x;
    //             let z = segment.z;
    //             let segment_index = x + z * 6;

    //             if segment_index > room_grid.len() - 1 {
    //                 bail!("Segment index for {},{} out of bounds: {}", x, z, segment_index);
    //             }
    //             if room_grid[segment_index] != 0 {
    //                 bail!("Segment at {},{} is already occupied by {}!", x, z, room_grid[segment_index]);
    //             }
    //             room_grid[segment_index] = room_index + 1;

    //             if x > grid_max_x {
    //                 grid_max_x = x;
    //             }
    //             if z > grid_max_y {
    //                 grid_max_y = z;
    //             }
    //         }
    //     }

    //     // I hate this so much
    //     for room in &rooms {
    //         let segments = &mut room.borrow_mut().segments;

    //         // this is to avoid borrow checking issues
    //         let segment_positions = segments.iter()
    //             .map(|segment| (segment.x, segment.z))
    //             .collect::<Vec<(usize, usize)>>();

    //         for segment in segments {
    //             let x = segment.x as isize;
    //             let z = segment.z as isize;
    //             let center_x = segment.x as i32 * 32 + 15 + DUNGEON_ORIGIN.0;
    //             let center_z = segment.z as i32 * 32 + 15 + DUNGEON_ORIGIN.1;

    //             let neighbour_options = [
    //                 (x, z - 1, center_x, center_z - 16),
    //                 (x + 1, z, center_x + 16, center_z),
    //                 (x, z + 1, center_x, center_z + 16),
    //                 (x - 1, z, center_x - 16, center_z),
    //             ];
    //             for (index, (nx, nz, door_x, door_z)) in neighbour_options.into_iter().enumerate() {
    //                 if nx < 0 || nz < 0 || segment_positions.iter().find(|(x, z)| *x as isize == nx && *z as isize == nz).is_some() {
    //                     continue;
    //                 }
    //                 let door = doors.iter().find(|door| {
    //                     door.borrow().x == door_x && door.borrow().z == door_z
    //                 });
    //                 if let Some(door) = door {
    //                     segment.neighbours[index] = Some(RoomNeighbour {
    //                         door: door.clone(),
    //                         room: rooms[room_grid[(nx + nz * 6) as usize] - 1].clone(),

    //                         door_index: 1, // todo: get index
    //                         room_index: room_grid[(nx + nz * 6) as usize]
    //                     });
    //                 }
    //             }
    //         }
    //     }

    //     let map_offset_x = (128 - (grid_max_x + 1) * 20) / 2;
    //     let map_offset_y = (128 - (grid_max_y + 1) * 20) / 2;

    //     Ok(Dungeon {
    //         server: std::ptr::null_mut(),
    //         door_storage: ArrayVec::new(),
    //         room_storage: ArrayVec::new(),
    //         rooms,
    //         doors,
    //         room_grid,
    //         new_room_grid: [const { None }; 36],
    //         state: DungeonState::NotReady,
    //         map: DungeonMap::new(map_offset_x, map_offset_y),
    //     })
    // }

    // // Layout String:
    // // 36 x room ids, two digits long each. 00 = no room, 01 -> 06 are special rooms like spawn, puzzles etc
    // // 07 -> ... are normal rooms, with unique ids to differentiate them and preserve layout
    // // Doors are 60x single digit numbers in the order left -> right top -> down for every spot they can possibly spawn
    // pub fn from_string(layout_str: &str, room_data_storage: &DeterministicHashMap<usize, RoomData>) -> anyhow::Result<Dungeon> {
    //     let mut rooms: Vec<Room> = Vec::new();
    //     // For normal rooms which can be larger than 1x1, store their segments and make the whole room in one go later
    //     let mut room_id_map: DeterministicHashMap<usize, Vec<RoomSegment>> = DeterministicHashMap::default();
    //     let mut doors: Vec<Door> = Vec::new();

    //     for (i, (x, z)) in DOOR_POSITIONS.into_iter().enumerate() {
    //         let type_str = layout_str.get(i + 72..i+73).unwrap();

    //         let door_type = match type_str {
    //             "0" => Some(DoorType::NORMAL),
    //             "1" => Some(DoorType::WITHER),
    //             "2" => Some(DoorType::BLOOD),
    //             "3" => Some(DoorType::ENTRANCE),
    //             _ => None,
    //         };

    //         if door_type.is_some() {
    //             // println!("{}", (x - DUNGEON_ORIGIN.0) / 16);
    //             let direction = match ((x - DUNGEON_ORIGIN.0) / 16) % 2 {
    //                 0 => Axis::Z,
    //                 1 => Axis::X,
    //                 _ => unreachable!(),
    //             };

    //             doors.push(Door {
    //                 x,
    //                 z,
    //                 direction,
    //                 door_type: door_type.unwrap()
    //             })
    //         }
    //     }

    //     for i in 0..36 {
    //         let substr = layout_str.get(i*2..i*2+2);
    //         let x = i % 6;
    //         let z = i / 6;

    //         // Shouldn't happen if data is not corrupted
    //         if substr.is_none() {
    //             panic!("Failed to parse dungeon string: too small.")
    //         }

    //         let id = substr.unwrap().parse::<usize>()?;

    //         // No room here
    //         if id == 0 {
    //             continue;
    //         }

    //         // Special rooms
    //         if id <= 6 {
    //             let room_type = match id {
    //                 1 => RoomType::Entrance,
    //                 2 => RoomType::Fairy,
    //                 3 => RoomType::Blood,
    //                 4 => RoomType::Puzzle,
    //                 5 => RoomType::Trap,
    //                 6 => RoomType::Yellow,
    //                 _ => unreachable!()
    //             };

    //             // Fairy can have a varying number of doors, all other special rooms are fixed to just one.
    //             let shape = match room_type {
    //                 RoomType::Fairy => RoomShape::OneByOne,
    //                 _ => RoomShape::OneByOneEnd,
    //             };

    //             let mut room_data = get_random_data_with_type(
    //                 room_type,
    //                 shape,
    //                 room_data_storage,
    //                 &rooms
    //             );

    //             room_data.room_type = room_type;

    //             rooms.push(Room::new(
    //                 vec![RoomSegment { x, z, neighbours: [const { None }; 4] }],
    //                 &doors,
    //                 room_data
    //             ));

    //             continue
    //         }

    //         // Normal rooms, add segments to this specific room id
    //         let entry = room_id_map.entry(id).or_default();
    //         entry.push(RoomSegment { x, z, neighbours: [const { None }; 4] });
    //     }

    //     // Make the normal rooms
    //     for (_, segments) in room_id_map {
    //         let shape = RoomShape::from_segments(&segments, &doors);

    //         rooms.push(Room::new(
    //             segments,
    //             &doors,
    //             get_random_data_with_type(
    //                 RoomType::Normal,
    //                 shape,
    //                 room_data_storage,
    //                 &rooms
    //             )
    //         ));
    //     }

    //     Dungeon::with_rooms_and_doors(rooms, doors)
    // }

    pub fn server_mut<'a>(&self) -> &'a mut Server {
        unsafe { self.server.as_mut().expect("server is null") }
    }

    pub fn get_room_at(&self, x: i32, z: i32) -> Option<usize> {
        if x < DUNGEON_ORIGIN.0 || z < DUNGEON_ORIGIN.1 {
            return None;
        }

        let grid_x = ((x - DUNGEON_ORIGIN.0) / 32) as usize;
        let grid_z = ((z - DUNGEON_ORIGIN.1) / 32) as usize;

        let entry = self.room_grid.get(grid_x + (grid_z * 6));
        entry.and_then(|e| *e)
    }
    
    pub fn get_player_room(&self, player: &Player) -> Option<usize> {
        self.get_room_at(
            player.position.x as i32,
            player.position.z as i32
        )
    }

    // /// Check if a player is inside the boss room
    // pub fn is_player_in_boss_room(&self, player: &Player) -> bool {
    //     let player_x = player.position.x as i32;
    //     let player_y = player.position.y as i32;
    //     let player_z = player.position.z as i32;
    //     
    //     // Check if player is within boss room bounds using stored dimensions
    //     // Boss room spans from Y=0 to Y=height (254), regardless of corner.y
    //     player_x >= self.boss_room_corner.x 
    //         && player_x < self.boss_room_corner.x + self.boss_room_width
    //         && player_z >= self.boss_room_corner.z 
    //         && player_z < self.boss_room_corner.z + self.boss_room_length
    //         && player_y >= 0  // Boss room starts from Y=0 (bottom)
    //         && player_y <= self.boss_room_height  // Up to the height of the room (254)
    // }

    pub fn start_dungeon(&mut self) {
        let world = &mut self.server_mut().world;
        for (index, door) in self.doors.iter().enumerate() {
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
        
        // Mark entrance room as entered and draw it on the map
        let mut entrance_room_index = None;
        for (room_index, room) in self.rooms.iter_mut().enumerate() {
            if room.room_data.room_type == crate::dungeon::room::room_data::RoomType::Entrance {
                if !room.entered {
                    room.entered = true;
                    entrance_room_index = Some(room_index);
                }
                break; // Only one entrance room, so we can break after finding it
            }
        }
        
        // Draw the entrance room on the map if it was just marked as entered
        if let Some(room_index) = entrance_room_index {
            self.map.draw_room(&self.rooms, &self.doors, room_index);
        }
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        let server = self.server_mut();

        match &mut self.state {
            DungeonState::NotReady | DungeonState::Finished => {}

            DungeonState::Starting { tick_countdown: tick } => {
                *tick -= 1;
                if *tick == 0 {
                    // Play final sounds when dungeon starts
                    // Play sounds 20 ticks after "Starting in 1 second" message
                    for (_, player) in &mut server.world.players {
                        // Ender dragon growl
                        let _ = player.write_packet(&SoundEffect {
                            sound: Sounds::EnderDragonGrowl.id(),
                            volume: 1.0,
                            pitch: 1.0,
                            pos_x: player.position.x,
                            pos_y: player.position.y,
                            pos_z: player.position.z,
                        });
                        
                        // Villager haggle
                        let _ = player.write_packet(&SoundEffect {
                            sound: Sounds::VillagerHaggle.id(),
                            volume: 1.0,
                            pitch: 0.7,
                            pos_x: player.position.x,
                            pos_y: player.position.y,
                            pos_z: player.position.z,
                        });
                    }
                    
                    // Send Mage stats message 20 ticks after "Starting in 1 second"
                    for (_, player) in &mut server.world.players {
                        player.send_message("§6Your Mage stats are doubled because");
                        player.send_message("§6you are the only player using this");
                        player.send_message("§6class!");
                        player.send_message("§a[Mage] §fIntelligence §c500 §f-> §a750");
                        player.send_message("§a[Mage] §fCooldown Reduction §c50% §f-> §a75%");
                    }
                    
                    // Send Mort message with slight delay after Mage stats
                    for (_, player) in &mut server.world.players {
                        player.send_message("§e[NPC] §bMort§f: Here, I found this map when I first entered the dungeon.");
                    }
                    
                    self.state = DungeonState::Started { current_ticks: 0 };
                    self.start_dungeon();
                } else if *tick % 20 == 0 {
                    let seconds_remaining = *tick / 20;
                    let s = if seconds_remaining == 1 { "" } else { "s" };
                    let str = format!("§aStarting in {} second{}.", seconds_remaining, s);

                    for (_, player) in &mut server.world.players {
                        player.send_message(&str);
                        
                        // Play random.click sound with specific volume and pitch during countdown
                        let _ = player.write_packet(&SoundEffect {
                            sound: Sounds::RandomClick.id(),
                            volume: 0.55,
                            pitch: 2.0,
                            pos_x: player.position.x,
                            pos_y: player.position.y,
                            pos_z: player.position.z,
                        });
                    }
                }
            }

            DungeonState::Started { current_ticks } => {
                *current_ticks += 1;
                
                // Play additional villager haggle sounds after the first one
                // 2000ms = 40 ticks after dungeon start (first additional sound)
                if *current_ticks == 40 {
                    for (_, player) in &mut server.world.players {
                        let _ = player.write_packet(&SoundEffect {
                            sound: Sounds::VillagerHaggle.id(),
                            volume: 1.0,
                            pitch: 0.7,
                            pos_x: player.position.x,
                            pos_y: player.position.y,
                            pos_z: player.position.z,
                        });
                    }
                    // Send Mort's follow-up message
                    for (_, player) in &mut server.world.players {
                        player.send_message("§e[NPC] §bMort§f: You should find it useful if you get lost.");
                    }
                }
                
                // 1500ms = 30 ticks after the second sound (70 ticks total)
                if *current_ticks == 70 {
                    for (_, player) in &mut server.world.players {
                        let _ = player.write_packet(&SoundEffect {
                            sound: Sounds::VillagerHaggle.id(),
                            volume: 1.0,
                            pitch: 0.7,
                            pos_x: player.position.x,
                            pos_y: player.position.y,
                            pos_z: player.position.z,
                        });
                    }
                    
                    // Send Mort's final message
                    for (_, player) in &mut server.world.players {
                        player.send_message("§e[NPC] §bMort§f: Good luck.");
                    }
                }
                
                // Play idle sounds for blood doors every second (20 ticks)
                if *current_ticks % 20 == 0 {
                    for door in &self.doors {
                        if door.door_type == DoorType::BLOOD {
                            // Check if the door is still interactable (not opened)
                            let door_center = BlockPos::new(door.x, 70, door.z);
                            if server.world.interactable_blocks.contains_key(&door_center) {
                                door.play_idle_sound(&mut server.world);
                            }
                        }
                    }
                }
                
                let mut room_index_and_player_ids: Vec<(usize, u32)> = Vec::new();
                // Collect all operations that need to happen on server.world, then execute them after the player loop
                let mut secrets_to_spawn_all: Vec<(std::rc::Rc<std::cell::RefCell<crate::dungeon::room::secrets::DungeonSecret>>, usize, u64)> = Vec::new();
                let mut bat_deaths_to_check_all: Vec<(std::rc::Rc<std::cell::RefCell<crate::dungeon::room::secrets::DungeonSecret>>, crate::server::entity::entity::EntityId, Option<u64>, usize)> = Vec::new();
                let mut room_secrets_found: HashMap<usize, u8> = HashMap::new();
                
                // First pass: collect room indices for each player (store positions to avoid borrow conflicts)
                // We need to calculate room indices outside the mutable borrow of self.state
                let room_grid = &self.room_grid; // Get immutable reference to room_grid before using it
                let mut player_data: Vec<(u32, f64, f64)> = Vec::new();
                for (player_id, player) in &server.world.players  {
                    player_data.push((*player_id, player.position.x, player.position.z));
                }
                
                // Get room indices for each player using the room_grid reference
                let mut player_room_indices: Vec<(u32, Option<usize>)> = Vec::new();
                for (player_id, x, z) in player_data {
                    // Calculate room index manually to avoid borrowing self
                    let x_i32 = x as i32;
                    let z_i32 = z as i32;
                    let room_index = if x_i32 < crate::dungeon::dungeon::DUNGEON_ORIGIN.0 || z_i32 < crate::dungeon::dungeon::DUNGEON_ORIGIN.1 {
                        None
                    } else {
                        let grid_x = ((x_i32 - crate::dungeon::dungeon::DUNGEON_ORIGIN.0) / 32) as usize;
                        let grid_z = ((z_i32 - crate::dungeon::dungeon::DUNGEON_ORIGIN.1) / 32) as usize;
                        room_grid.get(grid_x + (grid_z * 6)).and_then(|e| *e)
                    };
                    player_room_indices.push((player_id, room_index));
                }
                
                // Update player room indices
                for (player_id, room_index) in &player_room_indices {
                    if let Some(room_index) = room_index {
                        if let Some(p) = server.world.players.get_mut(player_id) {
                            p.current_room_index = Some(*room_index);
                        }
                    }
                }
                
                // Second pass: process each player's room
                // First, mark rooms as entered and collect entry secrets to spawn immediately (like locked chests)
                let mut rooms_just_entered: std::collections::HashSet<usize> = std::collections::HashSet::new();
                let mut entry_secrets_to_spawn: Vec<(std::rc::Rc<std::cell::RefCell<crate::dungeon::room::secrets::DungeonSecret>>, usize)> = Vec::new();
                
                for (player_id, room_index_opt) in &player_room_indices {
                    if let Some(room_index) = room_index_opt {
                        let room = self.rooms.get_mut(*room_index).unwrap();
                        if !room.entered {
                            room.entered = true;
                            rooms_just_entered.insert(*room_index);
                            
                            // Collect entry secrets (schest, sess) to spawn immediately when room is entered
                            if !room.room_entry_secrets_spawned {
                                room.room_entry_secrets_spawned = true;
                                for secret_rc in &room.json_secrets {
                                    let secret = secret_rc.borrow();
                                    if secret.has_spawned {
                                        continue;
                                    }
                                    
                                    match &secret.secret_type {
                                        crate::dungeon::room::secrets::SecretType::SecretChest { .. }
                                        | crate::dungeon::room::secrets::SecretType::SecretEssence => {
                                            entry_secrets_to_spawn.push((secret_rc.clone(), *room_index));
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Spawn entry secrets immediately (like locked chests)
                for (secret_rc, _room_index) in entry_secrets_to_spawn {
                    let mut secret = secret_rc.borrow_mut();
                    secret.has_spawned = true;
                    
                    crate::dungeon::room::secrets::DungeonSecret::spawn_into_world(
                        &secret_rc,
                        secret,
                        &mut server.world
                    );
                }
                
                // Collect player data (without ticking crushers)
                let mut player_room_data: Vec<(u32, usize, crate::server::utils::aabb::AABB)> = Vec::new();
                for (player_id, room_index_opt) in &player_room_indices {
                    if let Some(room_index) = room_index_opt {
                        let player = server.world.players.get_mut(player_id).unwrap();
                        let player_aabb = player.collision_aabb();
                        player_room_data.push((*player_id, *room_index, player_aabb));
                    }
                }
                
                // Tick crushers separately (they need mutable server access)
                for (player_id, room_index_opt) in &player_room_indices {
                    if let Some(room_index) = room_index_opt {
                        if let Some(player) = server.world.players.get_mut(player_id) {
                            let room = self.rooms.get_mut(*room_index).unwrap();
                            for crusher in room.crushers.iter_mut() {
                                crusher.tick(player);
                            }
                        }
                    }
                }
                
                // Now process secrets for each player's room
                for (player_id, room_index, player_aabb) in player_room_data {
                    let did_mark_entered = rooms_just_entered.contains(&room_index);
                        
                        // Tick secrets for this room
                        let is_secret_tick = *current_ticks % 20 == 0;
                        
                        // Collect secrets to spawn and bat IDs to check
                        let (proximity_secrets, bat_checks) = {
                            let room = self.rooms.get_mut(room_index).unwrap();
                            
                            let mut proximity_secrets = Vec::new();
                            let mut bat_checks = Vec::new();
                            
                            // Handle proximity-based secrets (rchest, ress, batsp, batdie, itemsp)
                            if is_secret_tick {
                                
                                for secret_rc in &room.json_secrets {
                                    let secret = secret_rc.borrow();
                                    if secret.has_spawned {
                                        continue;
                                    }
                                    
                                    // Check if player AABB intersects with secret's 8-block bounding box
                                    match &secret.secret_type {
                                        crate::dungeon::room::secrets::SecretType::RegularChest { .. }
                                        | crate::dungeon::room::secrets::SecretType::RegularEssence
                                        | crate::dungeon::room::secrets::SecretType::BatSpawn { .. }
                                        | crate::dungeon::room::secrets::SecretType::BatDie
                                        | crate::dungeon::room::secrets::SecretType::ItemSpawn { .. } => {
                                            if player_aabb.intersects(&secret.spawn_aabb) {
                                                proximity_secrets.push(secret_rc.clone());
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            
                            
                            // Collect bat tracking info
                            for secret_rc in &room.json_secrets {
                                let secret = secret_rc.borrow();
                                if let Some(bat_id) = secret.bat_entity_id {
                                    bat_checks.push((secret_rc.clone(), bat_id, secret.bat_spawn_tick));
                                }
                            }
                            
                            
                            (proximity_secrets, bat_checks)
                        };
                        
                        // Collect secrets to spawn (we'll execute after the player loop)
                        for secret_rc in proximity_secrets {
                            secrets_to_spawn_all.push((secret_rc, room_index, *current_ticks));
                        }
                        
                        // Collect bat checks (we'll execute after the player loop)
                        for (secret_rc, bat_id, spawn_tick) in bat_checks {
                            bat_deaths_to_check_all.push((secret_rc, bat_id, spawn_tick, room_index));
                        }
                        
                        // Draw room on map if just entered
                        if did_mark_entered {
                            self.map.draw_room(&self.rooms, &self.doors, room_index);

                            // Send map update to player
                            if let Some(player) = server.world.players.get_mut(&player_id) {
                                if let Some((region, data)) = self.map.get_updated_area() {
                                    let width = region.max_x - region.min_x;
                                    let height = region.max_y - region.min_y;

                                    player.write_packet(&Maps {
                                        id: 1,
                                        scale: 0,
                                        columns: width as u8,
                                        rows: height as u8,
                                        x: region.min_x as u8,
                                        z: region.min_y as u8,
                                        map_data: data,
                                    });
                                }
                            }
                        }
                        
                        // Register mushroom secret interactables for this player
                        let room_ref = &self.rooms[room_index];
                        if !room_ref.mushroom_sets.is_empty() {
                            // Store per-player up positions for set index resolution
                            let up_list: Vec<BlockPos> = room_ref.mushroom_sets.iter().map(|s| s.up.get(0).cloned().unwrap_or(BlockPos::new(0,0,0))).collect();
                            self.temp_player_mushroom_up.insert(player_id, up_list);

                            for (idx, set) in room_ref.mushroom_sets.iter().enumerate() {
                                for bp in &set.bottom {
                                    server.world.interactable_blocks.insert(*bp, BlockInteractAction::MushroomBottom { set_index: idx });
                                }
                                for bp in &set.top {
                                    server.world.interactable_blocks.insert(*bp, BlockInteractAction::MushroomTop);
                                }
                            }
                        }

                        room_index_and_player_ids.push((room_index, player_id));
                }
                
                // Now execute all world operations after the player loop is done
                // Spawn all secrets
                for (secret_rc, _room_index, current_ticks_val) in secrets_to_spawn_all {
                    let mut secret = secret_rc.borrow_mut();
                    secret.has_spawned = true;
                    
                    // Set spawn tick for batdie
                    if matches!(secret.secret_type, crate::dungeon::room::secrets::SecretType::BatDie) {
                        secret.bat_spawn_tick = Some(current_ticks_val);
                    }
                    
                    crate::dungeon::room::secrets::DungeonSecret::spawn_into_world(
                        &secret_rc,
                        secret,
                        &mut server.world
                    );
                }
                
                // Handle bat death tracking
                for (secret_rc, bat_id, spawn_tick, room_index) in bat_deaths_to_check_all {
                    let mut secret = secret_rc.borrow_mut();
                    
                    // Check batdie - kill bat after 5 ticks (0.25s)
                    if let Some(spawn_tick_val) = spawn_tick {
                        if matches!(secret.secret_type, crate::dungeon::room::secrets::SecretType::BatDie) {
                            if *current_ticks >= spawn_tick_val + 5 {
                                // Kill the bat and count as secret
                                if server.world.entities.contains_key(&bat_id) {
                                    // Get bat position before despawning for sound
                                    let bat_pos = if let Some((bat_entity, _)) = server.world.entities.get(&bat_id) {
                                        bat_entity.position
                                    } else {
                                        DVec3::new(
                                            secret.block_pos.x as f64 + 0.5,
                                            secret.block_pos.y as f64 + 0.5,
                                            secret.block_pos.z as f64 + 0.5
                                        )
                                    };
                                    
                                    // Play bat death sound
                                    for (_, player) in &mut server.world.players {
                                        let _ = player.write_packet(&crate::net::protocol::play::clientbound::SoundEffect {
                                            sound: crate::server::utils::sounds::Sounds::BatDeath.id(),
                                            pos_x: bat_pos.x,
                                            pos_y: bat_pos.y,
                                            pos_z: bat_pos.z,
                                            volume: 1.0,
                                            pitch: 1.0,
                                        });
                                    }
                                    
                                    // Despawn the bat
                                    server.world.despawn_entity(bat_id);
                                    
                                    // Count as secret
                                    if !secret.obtained {
                                        secret.obtained = true;
                                        *room_secrets_found.entry(room_index).or_insert(0u8) += 1u8;
                                    }
                                }
                                secret.bat_entity_id = None;
                            }
                        }
                    }
                    
                    // Check batsp - if bat died naturally, count as secret
                    if matches!(secret.secret_type, crate::dungeon::room::secrets::SecretType::BatSpawn { .. }) {
                        if !server.world.entities.contains_key(&bat_id) {
                            // Bat died naturally
                            if !secret.obtained {
                                secret.obtained = true;
                                *room_secrets_found.entry(room_index).or_insert(0u8) += 1u8;
                            }
                            secret.bat_entity_id = None;
                        }
                    }
                }
                
                // Check for ItemSpawn secrets that have been obtained (only count once)
                for room_index in 0..self.rooms.len() {
                    let room = &self.rooms[room_index];
                    for secret_rc in &room.json_secrets {
                        let mut secret = secret_rc.borrow_mut();
                        if matches!(secret.secret_type, crate::dungeon::room::secrets::SecretType::ItemSpawn { .. })
                            && secret.obtained && secret.has_spawned && !secret.counted {
                            // Count it once and mark as counted
                            secret.counted = true;
                            *room_secrets_found.entry(room_index).or_insert(0u8) += 1u8;
                        }
                    }
                }
                
                // Update found_secrets count for all rooms
                for (room_index, count) in room_secrets_found.into_iter() {
                    let count_u8: u8 = count; // Explicitly type as u8
                    let room = self.rooms.get_mut(room_index).unwrap();
                    let current: u16 = room.found_secrets as u16;
                    let add: u16 = count_u8 as u16;
                    let new_count: u8 = current.saturating_add(add).min(255) as u8;
                    room.found_secrets = new_count;
                }

                // After loop, send debug per (room, player) without borrow conflicts
                for (room_index, player_id) in room_index_and_player_ids.into_iter() {
                    let (name, shape, rotation, pattern_len) = {
                        let room = self.rooms.get(room_index).unwrap();
                        if !(room.entered && room.crypt_patterns.len() > 0 && !room.crypts_checked) {
                            continue;
                        }
                        let name = room.room_data.name.clone();
                        let shape = room.room_data.shape.clone();
                        let rotation = room.rotation;
                        let pattern_len = room.crypt_patterns.len();
                        (name, shape, rotation, pattern_len)
                    };

                    let count = {
                        let room_mut = self.rooms.get_mut(room_index).unwrap();
                        room_mut.detect_crypts(&server.world)
                    };

                    // Also detect superboomwalls
                    let walls_count = {
                        let room_mut = self.rooms.get_mut(room_index).unwrap();
                        room_mut.detect_superboomwalls(&server.world)
                    };

                    // Also detect falling blocks
                    let fallingblocks_count = {
                        let room_mut = self.rooms.get_mut(room_index).unwrap();
                        room_mut.detect_fallingblocks(&server.world)
                    };

                    if let Some(player) = server.world.players.get_mut(&player_id) {
                        player.send_message(&format!(
                            "§8[crypts] §7room §f{}§7 shape {:?} rotation {:?} — patterns: §e{}§7, matched: §a{}",
                            name,
                            shape,
                            rotation,
                            pattern_len,
                            count
                        ));
                        
                        // Also show superboomwalls info
                        let walls_pattern_len = {
                            let room = self.rooms.get(room_index).unwrap();
                            room.superboomwall_patterns.len()
                        };
                        player.send_message(&format!(
                            "§8[walls] §7room §f{}§7 — superboomwall patterns: §e{}§7, matched: §a{}",
                            name,
                            walls_pattern_len,
                            walls_count
                        ));
                        
                        // Also show falling blocks info
                        let fallingblocks_pattern_len = {
                            let room = self.rooms.get(room_index).unwrap();
                            room.fallingblock_patterns.len()
                        };
                        player.send_message(&format!(
                            "§8[falling] §7room §f{}§7 — falling block patterns: §e{}§7, matched: §a{}",
                            name,
                            fallingblocks_pattern_len,
                            fallingblocks_count
                        ));
                    }
                }
            }
        }

        // Tick all rooms to process falling blocks
        for room in self.rooms.iter_mut() {
            room.tick(&mut server.world);
        }

        Ok(())
    }

    /// Superboom TNT logic: find the room containing `pos` and explode crypts and superboomwalls.
    /// For crypts: explodes all patterns within radius.
    /// For superboomwalls: explodes only the FIRST pattern within radius (one at a time).
    /// Plays explosion sound for all players.
    pub fn superboom_at(&mut self, pos: BlockPos, radius: i32) -> anyhow::Result<()> {
        if let Some(room_index) = self.get_room_at(pos.x, pos.z) {
            let server = self.server_mut();
            let world = &mut server.world;
            let room = self.rooms.get_mut(room_index).unwrap();
            
            // Explode crypts (all patterns within radius)
            let crypts_exploded = room.explode_crypt_near(world, &pos, radius);
            
            // Explode superboomwalls (only first pattern within radius)
            let walls_exploded = room.explode_superboomwall_near(world, &pos, radius);
            
            if crypts_exploded > 0 || walls_exploded > 0 {
                // Play explosion sound for all players
                for (_, player) in &mut world.players {
                    let _ = player.write_packet(&SoundEffect {
                        sound: Sounds::RandomExplode.id(),
                        volume: 1.0,
                        pitch: 1.0,
                        pos_x: pos.x as f64 + 0.5,
                        pos_y: pos.y as f64 + 0.5,
                        pos_z: pos.z as f64 + 0.5,
                    });
                }
                
            }
        }
        Ok(())
    }
}

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

    pub fn get_room_at(&mut self, x: i32, z: i32) -> Option<usize> {
        if x < DUNGEON_ORIGIN.0 || z < DUNGEON_ORIGIN.1 {
            return None;
        }

        let grid_x = ((x - DUNGEON_ORIGIN.0) / 32) as usize;
        let grid_z = ((z - DUNGEON_ORIGIN.1) / 32) as usize;

        let entry = self.room_grid.get(grid_x + (grid_z * 6));
        entry.and_then(|e| *e)
    }
    
    pub fn get_player_room(&mut self, player: &Player) -> Option<usize> {
        self.get_room_at(
            player.position.x as i32,
            player.position.z as i32
        )
    }

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
        // probably mark room connected to entrance as entered
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
                for (player_id, player) in &mut server.world.players  {
                    if let Some(room_index) = self.get_player_room(player) {
                        // Limit mutable borrow scope to avoid conflicts with immutable borrows later
                        let mut did_mark_entered = false;
                        {
                            let room = self.rooms.get_mut(room_index).unwrap();

                            for crusher in room.crushers.iter_mut() {
                                crusher.tick(player);
                            }

                            if !room.entered {
                                room.entered = true;
                                did_mark_entered = true;
                            }
                        }

                        if did_mark_entered {
                            self.map.draw_room(&self.rooms, &self.doors, room_index);

                            // this needs to happen once a tick,
                            // but currently the ticking stuff is a mess
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
                            };
                        }

                        // Register mushroom secret interactables for this player: store up list, and insert bottom/top positions as interactables
                        let room_ref = &self.rooms[room_index];
                        if !room_ref.mushroom_sets.is_empty() {
                            // Store per-player up positions for set index resolution
                            let up_list: Vec<BlockPos> = room_ref.mushroom_sets.iter().map(|s| s.up.get(0).cloned().unwrap_or(BlockPos::new(0,0,0))).collect();
                            self.temp_player_mushroom_up.insert(*player_id, up_list);

                            for (idx, set) in room_ref.mushroom_sets.iter().enumerate() {
                                for bp in &set.bottom {
                                    server.world.interactable_blocks.insert(*bp, BlockInteractAction::MushroomBottom { set_index: idx });
                                }
                                for bp in &set.top {
                                    server.world.interactable_blocks.insert(*bp, BlockInteractAction::MushroomTop);
                                }
                            }
                        }

                        room_index_and_player_ids.push((room_index, *player_id));
                    }
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

                    if let Some(player) = server.world.players.get_mut(&player_id) {
                        player.send_message(&format!(
                            "§8[crypts] §7room §f{}§7 shape {:?} rotation {:?} — patterns: §e{}§7, matched: §a{}",
                            name,
                            shape,
                            rotation,
                            pattern_len,
                            count
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Superboom TNT logic: find the room containing `pos` and explode any crypt pattern
    /// that has a block within a radius of 2 from `pos`. Plays explosion sound for all players.
    pub fn superboom_at(&mut self, pos: BlockPos, radius: i32) -> anyhow::Result<()> {
        if let Some(room_index) = self.get_room_at(pos.x, pos.z) {
            let server = self.server_mut();
            let world = &mut server.world;
            let room = self.rooms.get_mut(room_index).unwrap();
            let exploded_count = room.explode_crypt_near(world, &pos, radius);
            if exploded_count > 0 {
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

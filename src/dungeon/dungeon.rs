use crate::dungeon::door::{Door, DoorType};
use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::map::DungeonMap;
use crate::dungeon::room::room::{Room, RoomNeighbour, RoomSegment};
use crate::dungeon::room::room_data::{get_random_data_with_type, RoomData, RoomShape, RoomType};
use crate::dungeon::score::DungeonScoreState;
use crate::net::protocol::play::clientbound::{Maps, MapIcon};
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_parameter::Axis;
use crate::server::block::block_position::BlockPos;
use crate::server::entity::dungeon_mobs::spawn_room_mobs;
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::dungeon::room::secrets::{PickupEntityImpl, PickupKind};
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

/// Spawns one `PickupEntityImpl` (Wither Key, Blood Key, or Superboom TNT) at `pos` plus its
/// following nametag. Shared by `Dungeon::maybe_grant_door_key`'s method and its inlined copy in
/// `tick()`'s room-discovery loop (both need identical spawn+nametag boilerplate for the key
/// itself and its TNT companion).
pub(crate) fn spawn_pickup(world: &mut world::World, pos: DVec3, kind: PickupKind) {
    if let Ok(entity_id) = world.spawn_entity(
        pos,
        {
            let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
            metadata.is_invisible = true;
            metadata
        },
        PickupEntityImpl { kind },
    ) {
        let _ = crate::server::entity::spawn_equipped::spawn_following_nametag(
            world,
            entity_id,
            kind.colored_name(),
            1.5,
            EntityVariant::ArmorStand,
        );
    }
}

// The top leftmost corner of the dungeon
pub const DUNGEON_ORIGIN: (i32, i32) = (-200, -200);

/// A room's mobs don't spawn the instant the room is entered - they queue in
/// `Dungeon::pending_mob_spawn_rooms` and only actually spawn on a tick where
/// `current_ticks % MOB_SPAWN_CYCLE_TICKS == 0`. This cycle runs continuously off the dungeon's
/// own tick counter for as long as it stays `Started` (not reset per-room), so a room entered
/// mid-cycle waits for the next 0 rather than spawning immediately - see the `Started` arm of
/// `Dungeon::tick`.
pub const MOB_SPAWN_CYCLE_TICKS: u64 = 5;

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

    /// Set on dungeons built by `dungeon::practice` (see `practice::apply_practice_dungeon`).
    /// Suppresses mob spawning and Mort's dungeon-start flavor text/sounds, neither of which
    /// make sense for an isolated single-room practice arena.
    pub practice_room: bool,

    /// How many secrets `/practice <room> <door> <secrets>` was asked to route for - the route
    /// timer (see `tick`, below) finishes once `rooms[0].found_secrets` reaches this. Carried
    /// forward across `/rs` (only the timer/finished-latch reset, not the target) so a player
    /// can retry the same route target repeatedly. `None` means no practice room is loaded yet.
    pub practice_target_secrets: Option<u8>,
    /// Whether `/practice ... as` asked every secret in the room to be force-spawned immediately
    /// (see `practice::spawn_all_secrets`), instead of the normal proximity/room-entry gating.
    /// Carried across `/rs` like `practice_target_secrets`, so a restart repeats the same choice.
    pub practice_instant_secrets: bool,
    /// Tick the route timer started on - set the first time any player's position moves away
    /// from where they spawned into the room (see `tick`), i.e. "first input", not room-load
    /// time. `None` before that first movement.
    pub practice_route_start_tick: Option<u64>,
    /// Latches once the finish time has been announced, so it isn't re-sent every tick
    /// afterward (e.g. if more secrets keep getting found past the target).
    pub practice_route_finished: bool,
    /// Elapsed seconds at the moment the route finished - the action bar freezes on this
    /// instead of continuing to count once `practice_route_finished` is set.
    pub practice_route_finish_seconds: Option<f64>,
    /// Precomputed compact label for the finish chat message, e.g. "Catwalk E5/6" (room name,
    /// door letter + target secrets, room's total secrets). Carried across `/rs` like the
    /// target itself.
    pub practice_route_prefix: Option<String>,
    /// `rooms[0].found_secrets` as of the previous tick, so an increase (a secret just found)
    /// can be detected to trigger the action bar's green flash.
    pub practice_last_found_secrets: u8,
    /// Ticks remaining to render the live action bar timer in the "flash" color after a secret
    /// was just found.
    pub practice_route_flash_ticks: u32,
    /// Legacy (`&`-coded) text for the live route timer, e.g. `"&612.45s"` - folded into the
    /// existing HP/Defense/Mana/Secrets action bar by `main.rs` rather than sent as its own
    /// packet (see `tick`). `None` when no practice route is armed.
    pub practice_timer_text: Option<String>,

    /// F7 score tracking (skill/exploration/speed/bonus) + S/S+ announcement state. Reset
    /// whenever a run starts (see the `Started` transition below).
    pub score: DungeonScoreState,

    // Temporary per-player mapping of mushroom set index -> up destination (world BlockPos)
    pub temp_player_mushroom_up: HashMap<u32, Vec<BlockPos>>,
    
    // Locked chest system
    // Maps chest world position to its locked state and associated lever
    pub locked_chests: HashMap<BlockPos, LockedChestState>,
    // Maps lever world position to all chests it unlocks
    pub lever_to_chests: HashMap<BlockPos, Vec<BlockPos>>,

    /// Toggled from the magical map's right-click GUI (see `UI::MapSettingsMenu`). When true,
    /// every secret in a room spawns the instant the room is entered, instead of the normal
    /// per-secret bounding-box proximity gating in `tick` below. Off by default so ordinary runs
    /// keep vanilla secret-spawn behavior.
    pub secrets_always_spawn: bool,

    /// Room indices that have been entered but whose mobs haven't spawned yet - drained once
    /// per `MOB_SPAWN_CYCLE_TICKS` ticks (see `tick`'s `Started` arm), instead of spawning the
    /// instant a room is entered.
    pub pending_mob_spawn_rooms: Vec<usize>,

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
                    
                    // In a normal generated layout a door always has a real room on both
                    // sides, so `room_grid` is always populated here. Practice mode
                    // (`dungeon::practice`) is the one case that isn't true: a practice room's
                    // doors intentionally face outward into an otherwise-empty grid cell (there
                    // is no dungeon on the other side to walk into), so skip wiring a neighbour
                    // rather than assuming one exists.
                    if let Some((door_index, _)) = door {
                        if let Some(room_index) = room_grid[(nx + nz * 6) as usize] {
                            segment.neighbours[index] = Some(RoomNeighbour {
                                door_index,
                                room_index,
                            });
                        }
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
            practice_room: false,
            practice_target_secrets: None,
            practice_instant_secrets: false,
            practice_route_start_tick: None,
            practice_route_finished: false,
            practice_route_finish_seconds: None,
            practice_route_prefix: None,
            practice_last_found_secrets: 0,
            practice_route_flash_ticks: 0,
            practice_timer_text: None,
            score: DungeonScoreState::default(),
            temp_player_mushroom_up: HashMap::new(),
            locked_chests: HashMap::new(),
            lever_to_chests: HashMap::new(),
            secrets_always_spawn: false,
            pending_mob_spawn_rooms: Vec::new(),
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
                    door_type,
                    key_granted: false,
                    opened: false,
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

    /// Force-spawns every not-yet-spawned secret in every already-entered room, bypassing the
    /// normal bounding-box/room-entry gating. Used when `secrets_always_spawn` is toggled on
    /// (see `UI::MapSettingsMenu`) so rooms the players are already standing in catch up
    /// immediately instead of waiting for the next room entry.
    pub fn spawn_all_secrets_in_entered_rooms(&mut self, world: &mut world::World) {
        let mut secrets_to_spawn = Vec::new();
        for room in &self.rooms {
            if !room.entered {
                continue;
            }
            secrets_to_spawn.extend(room.json_secrets.iter().cloned());
        }

        for secret_rc in secrets_to_spawn {
            let mut secret = secret_rc.borrow_mut();
            if secret.has_spawned {
                continue;
            }
            secret.has_spawned = true;
            crate::dungeon::room::secrets::DungeonSecret::spawn_into_world(&secret_rc, secret, world);
        }

        for room in &mut self.rooms {
            if room.entered {
                room.room_entry_secrets_spawned = true;
            }
        }
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

    /// Update the map for a specific room and send the update to all players
    pub fn update_map_for_room(&mut self, room_index: usize) {
        let server = self.server_mut();
        
        // Redraw the room on the map
        self.map.draw_room(&self.rooms, &self.doors, room_index);
        
        // Send map update to all players
        if let Some((region, data)) = self.map.get_updated_area() {
            let width = region.max_x - region.min_x;
            let height = region.max_y - region.min_y;
            
            for (_, player) in &mut server.world.players {
                player.write_packet(&Maps {
                    id: 1,
                    scale: 0,
                    icons: vec![],
                    columns: width as u8,
                    rows: height as u8,
                    x: region.min_x as u8,
                    z: region.min_y as u8,
                    map_data: data.clone(),
                });
            }
        }
    }

    /// Checks whether `room_index`'s starred mobs are all dead and, if the room leads to a
    /// not-yet-granted door of `kind`'s type (Wither or Blood - never call this with `Tnt`, it
    /// has no door of its own and is spawned as a companion below instead), spawns that key plus
    /// a Superboom TNT right next to it. Called both from `combat.rs::kill_mob` right after a
    /// room's last starred mob dies - passing `death_pos` so the key spawns where that mob
    /// actually died, not at the door - and from the room-discovery loop below for rooms that
    /// start at 0 starred mobs (nothing died, so `death_pos` is `None` and the key spawns in the
    /// room instead, the only position that makes sense when there was nothing to kill).
    pub fn maybe_grant_door_key(&mut self, room_index: usize, death_pos: Option<DVec3>, kind: PickupKind) {
        let Some(room) = self.rooms.get(room_index) else { return; };
        if room.starred_mobs_remaining != 0 {
            return;
        }
        let Some(wanted_door_type) = kind.door_type() else { return; };

        // Also remembers which segment matched, not just the door - a spawn position built from
        // the door's own coordinates lands literally inside the door frame (confirmed by
        // testing), so the no-starred-mobs fallback below uses the segment's world position
        // instead, landing somewhere inside the actual room rather than in the doorway.
        let mut matched_door: Option<(usize, usize, usize)> = None; // (door_index, segment_x, segment_z)
        'find_door: for segment in &room.segments {
            for neighbour in segment.neighbours.iter().flatten() {
                if let Some(door) = self.doors.get(neighbour.door_index) {
                    if door.door_type == wanted_door_type && !door.key_granted {
                        matched_door = Some((neighbour.door_index, segment.x, segment.z));
                        break 'find_door;
                    }
                }
            }
        }

        let Some((door_index, segment_x, segment_z)) = matched_door else { return; };
        self.doors[door_index].key_granted = true;
        let mut spawn_pos = death_pos.unwrap_or_else(|| {
            // Center of the matched 32x32 room segment - same grid math `main.rs`/`dungeon.rs`
            // use elsewhere to turn a world position into a segment index, just inverted.
            DVec3::new(
                DUNGEON_ORIGIN.0 as f64 + segment_x as f64 * 32.0 + 16.0,
                71.0,
                DUNGEON_ORIGIN.1 as f64 + segment_z as f64 * 32.0 + 16.0,
            )
        });
        spawn_pos.y -= 1.0;

        let server = self.server_mut();
        spawn_pickup(&mut server.world, spawn_pos, kind);
        // TNT always accompanies whichever key was just granted - "right on the block next to
        // it" per spec, so a flat 1-block nudge on X is enough distinction without needing any
        // room-shape awareness.
        let tnt_pos = DVec3::new(spawn_pos.x + 1.0, spawn_pos.y, spawn_pos.z);
        spawn_pickup(&mut server.world, tnt_pos, PickupKind::Tnt);
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

    /// Finds the door index of whichever door sits on the shortest room-graph path from the
    /// entrance to the Fairy room - i.e. the door you'd naturally walk through to *reach*
    /// Fairy, as opposed to any other door touching it that leads further/deeper into the
    /// dungeon. Fairy is the only special room that can have more than one door (see
    /// `from_str`), so unlike every other special room this can't just be "the one door".
    ///
    /// Plain BFS over the room graph (`RoomSegment::neighbours`) from the entrance - since
    /// dungeon layouts are effectively tree-shaped, the first time Fairy is reached is via its
    /// shortest (and in practice only sensible) path in from the explored side.
    fn find_fairy_entry_door(&self) -> Option<usize> {
        let entrance_index = self.rooms.iter().position(|r| r.room_data.room_type == RoomType::Entrance)?;
        let fairy_index = self.rooms.iter().position(|r| r.room_data.room_type == RoomType::Fairy)?;

        let mut visited = vec![false; self.rooms.len()];
        let mut entry_door: Vec<Option<usize>> = vec![None; self.rooms.len()];
        let mut queue = std::collections::VecDeque::new();
        visited[entrance_index] = true;
        queue.push_back(entrance_index);

        while let Some(current) = queue.pop_front() {
            if current == fairy_index {
                return entry_door[current];
            }
            for segment in &self.rooms[current].segments {
                for neighbour in segment.neighbours.iter().flatten() {
                    if !visited[neighbour.room_index] {
                        visited[neighbour.room_index] = true;
                        entry_door[neighbour.room_index] = Some(neighbour.door_index);
                        queue.push_back(neighbour.room_index);
                    }
                }
            }
        }

        None
    }

    pub fn start_dungeon(&mut self) {
        // Computed before any door gets touched below, so the loop can special-case exactly
        // this one door - see `find_fairy_entry_door`.
        let fairy_entry_door = self.find_fairy_entry_door();

        let world = &mut self.server_mut().world;
        // Indexed loop rather than `self.doors.iter()` - `opened` below needs a fresh `&mut
        // self.doors[index]` each iteration, which can't coexist with a single long-lived
        // iterator borrow of the whole Vec.
        for index in 0..self.doors.len() {
            let door = &self.doors[index];
            if door.door_type == DoorType::ENTRANCE {
                door.open_door(world);
                self.doors[index].opened = true;
                continue;
            }

            // The wither door leading INTO the Fairy room (shortest path from the entrance)
            // starts open just like the entrance door - any other door touching Fairy (the way
            // further out/deeper into the dungeon) stays a normal locked wither door.
            if door.door_type == DoorType::WITHER && fairy_entry_door == Some(index) {
                door.open_door(world);
                self.doors[index].opened = true;
                continue;
            }

            let door = &self.doors[index];
            if door.door_type == DoorType::NORMAL {
                continue;
            }

            // Covers the door's full 5x5x5 frame (see `Door::load_into_world`'s block-placement
            // loop, which places `chosen`'s 125 blocks across x/z -2..=2 and y 69..=73 around
            // the door center) - not just the inner opening - so clicking anywhere on the
            // visible door, frame included, opens it.
            world::iterate_blocks(
                BlockPos { x: door.x - 2, y: 69, z: door.z - 2 },
                BlockPos { x: door.x + 2, y: 73, z: door.z + 2 },
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
            // Queue this room's mobs to spawn on the next mob-spawn cycle tick (see
            // `MOB_SPAWN_CYCLE_TICKS`/`tick`'s `Started` arm), rather than spawning immediately.
            if !self.practice_room {
                self.pending_mob_spawn_rooms.push(room_index);
            }
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
                    
                    self.score.reset();
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

                // Every player's own position/facing arrow on the magical map - unlike the room
                // pixel data (only resent when something on the map actually changes), this has
                // to be pushed every tick since the player is presumably moving continuously.
                // `columns: 0` means no pixel data is included, just the icon list (see
                // `Maps::write`) - a real client-holds-a-filled-map update, not a room redraw.
                {
                    let icons: Vec<MapIcon> = server.world.players.values().map(|player| {
                        let (x, z) = self.map.world_to_icon(player.position.x, player.position.z, DUNGEON_ORIGIN);
                        // `yaw` is degrees, sign/range varies by how it was last set - normalize
                        // into 0..360 before quantizing to one of the 16 directions.
                        let normalized_yaw = ((player.yaw % 360.0) + 360.0) % 360.0;
                        let direction = ((normalized_yaw / 22.5).round() as u8) & 0x0F;
                        MapIcon { icon_type: 0, direction, x, z }
                    }).collect();

                    for (_, player) in &mut server.world.players {
                        player.write_packet(&Maps {
                            id: 1,
                            scale: 0,
                            icons: icons.clone(),
                            columns: 0,
                            rows: 0,
                            x: 0,
                            z: 0,
                            map_data: Vec::new(),
                        });
                    }
                }

                // Score is re-derived from live room state every tick (covers room clears and
                // secrets found - both already tracked authoritatively on `Room` - without a
                // second, independently-incremented copy that could drift out of sync) and
                // checked against the S/S+ thresholds; `check_score_announcements` itself is a
                // one-shot latch so this is safe to call unconditionally every tick.
                self.score.elapsed_ticks = *current_ticks;
                self.score.sync_exploration(&self.rooms);
                self.score.check_score_announcements(&mut server.world);

                // Practice-mode secret route timer: starts on the first tick any player's
                // position has moved away from where they spawned into the room ("first
                // input"), finishes once the room's found secrets reach the requested target.
                // While active, every connected player gets a live action-bar timer that
                // flashes green for a moment whenever a secret is found, then freezes on the
                // final time once the route finishes.
                if self.practice_room {
                    if let Some(target) = self.practice_target_secrets {
                        if self.practice_route_start_tick.is_none() {
                            let moved = server.world.players.values().any(|player| {
                                let Some((spawn_pos, _, _)) = player.practice_last_spawn else { return false };
                                let dx = player.position.x - spawn_pos.x;
                                let dy = player.position.y - spawn_pos.y;
                                let dz = player.position.z - spawn_pos.z;
                                dx * dx + dy * dy + dz * dz > 0.0004 // moved more than ~2cm
                            });
                            if moved {
                                self.practice_route_start_tick = Some(*current_ticks);
                            }
                        }

                        let found = self.rooms.get(0).map(|room| room.found_secrets).unwrap_or(0);
                        if found > self.practice_last_found_secrets {
                            const FLASH_TICKS: u32 = 10; // ~0.5s
                            self.practice_route_flash_ticks = FLASH_TICKS;
                        }
                        self.practice_last_found_secrets = found;
                        self.practice_route_flash_ticks = self.practice_route_flash_ticks.saturating_sub(1);

                        if !self.practice_route_finished {
                            if let Some(start_tick) = self.practice_route_start_tick {
                                if found as u16 >= target as u16 {
                                    self.practice_route_finished = true;
                                    let seconds = current_ticks.saturating_sub(start_tick) as f64 * 0.05;
                                    self.practice_route_finish_seconds = Some(seconds);
                                    let prefix = self.practice_route_prefix.as_deref().unwrap_or("Practice");
                                    let message = format!("\u{a7}f{}  \u{a7}fTime: \u{a7}a{:.2}s", prefix, seconds);
                                    for (_, player) in &mut server.world.players {
                                        player.send_message(&message);
                                    }
                                }
                            }
                        }

                        // Live timer text, folded into the existing HP/Defense/Mana/Secrets
                        // action bar (see `main.rs`'s per-player tick loop) instead of being
                        // sent as its own competing action-bar packet - only one action-bar
                        // message can be shown at a time, so a separate packet here would just
                        // flicker/overwrite the normal HUD instead of sitting alongside it.
                        // Frozen on the final time once finished, otherwise a live count-up
                        // from the start tick (or 0.00s if not started yet). Gold (`&6`, "MC
                        // color 6") normally, green while flashing after a secret or once done.
                        let seconds = if self.practice_route_finished {
                            self.practice_route_finish_seconds.unwrap_or(0.0)
                        } else if let Some(start_tick) = self.practice_route_start_tick {
                            current_ticks.saturating_sub(start_tick) as f64 * 0.05
                        } else {
                            0.0
                        };
                        let color = if self.practice_route_finished || self.practice_route_flash_ticks > 0 {
                            "&a" // green: done, or briefly after a secret is found
                        } else {
                            "&6" // gold
                        };
                        self.practice_timer_text = Some(format!("{}{:.2}s", color, seconds));
                    }
                }

                // Play additional villager haggle sounds after the first one
                // 2000ms = 40 ticks after dungeon start (first additional sound)
                // No Mort in a practice room, so skip his flavor text/sounds entirely.
                if *current_ticks == 40 && !self.practice_room {
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
                if *current_ticks == 70 && !self.practice_room {
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
                            
                            // Collect entry secrets (schest, sess) to spawn immediately when room is entered.
                            // With `secrets_always_spawn` on, every secret in the room spawns on entry
                            // instead of just schest/sess, bypassing the proximity gating below entirely.
                            if !room.room_entry_secrets_spawned {
                                room.room_entry_secrets_spawned = true;
                                for secret_rc in &room.json_secrets {
                                    let secret = secret_rc.borrow();
                                    if secret.has_spawned {
                                        continue;
                                    }

                                    if self.secrets_always_spawn {
                                        entry_secrets_to_spawn.push((secret_rc.clone(), *room_index));
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
                
                // Rooms entered this tick queue behind the same mob-spawn cycle as any room
                // already pending (e.g. the entrance room, queued in `start_dungeon`) - mobs
                // only actually spawn on a tick where `current_ticks % MOB_SPAWN_CYCLE_TICKS ==
                // 0`, so a room entered mid-cycle waits for the next cycle point instead of
                // spawning the instant it's entered. The cycle runs continuously off
                // `current_ticks` for as long as the dungeon stays `Started` - it's never reset
                // per-room, so it doesn't restart just because a new room got queued.
                let mut rooms_spawned_this_tick: Vec<usize> = Vec::new();
                if !self.practice_room {
                    self.pending_mob_spawn_rooms.extend(rooms_just_entered.iter().copied());
                    if *current_ticks % MOB_SPAWN_CYCLE_TICKS == 0 {
                        rooms_spawned_this_tick = std::mem::take(&mut self.pending_mob_spawn_rooms);
                        for &room_index in &rooms_spawned_this_tick {
                            if let Some(room) = self.rooms.get(room_index) {
                                spawn_room_mobs(&mut server.world, room_index, room);
                            }
                            if let Some(room) = self.rooms.get_mut(room_index) {
                                room.mobs_spawned = true;
                            }

                            // Redraw now that `mobs_spawned` (and, for a room with no starred
                            // mobs, `starred_mobs_remaining == 0`) is finally accurate - the
                            // room-entry draw earlier (see `did_mark_entered` below) may have
                            // already happened on an earlier tick, before this room's mobs
                            // existed, so its checkmark can't rely on that call alone. Inlined
                            // rather than calling `update_map_for_room` - `match &mut self.state`
                            // above already holds `self` mutably borrowed for this whole arm.
                            self.map.draw_room(&self.rooms, &self.doors, room_index);
                            if let Some((region, data)) = self.map.get_updated_area() {
                                let width = region.max_x - region.min_x;
                                let height = region.max_y - region.min_y;
                                for (_, player) in &mut server.world.players {
                                    player.write_packet(&Maps {
                                        id: 1,
                                        scale: 0,
                                        icons: vec![],
                                        columns: width as u8,
                                        rows: height as u8,
                                        x: region.min_x as u8,
                                        z: region.min_y as u8,
                                        map_data: data.clone(),
                                    });
                                }
                            }
                        }
                    }
                }

                // Rooms with no starred mobs at all (so `spawn_room_mobs` above never gave them
                // any to kill) leading to a Wither or Blood Door get their key granted immediately
                // instead of waiting on a kill event that will never happen - rooms that DO have
                // starred mobs are handled instead in `combat.rs::kill_mob` once the last one dies.
                // Inlined rather than calling `maybe_grant_door_key` (same logic, extracted for
                // the `combat.rs` call site) - `match &mut self.state` above is already holding
                // `self` mutably borrowed here, and a method call on `self` would conflict with
                // that, the same borrow-conflict class as the chest-particle code just below.
                // Keyed off `rooms_spawned_this_tick`, NOT `rooms_just_entered` - mobs may not
                // have actually spawned into a just-entered room yet (still queued behind the
                // cycle gate above), and `starred_mobs_remaining` would still misleadingly read 0
                // for a room that does have starred mobs coming, wrongly granting the key early.
                for &room_index in &rooms_spawned_this_tick {
                    for kind in [PickupKind::Wither, PickupKind::Blood] {
                        if let Some(room) = self.rooms.get(room_index) {
                            if room.starred_mobs_remaining == 0 {
                                let wanted_door_type = kind.door_type().expect("Wither/Blood always have a door_type");
                                let mut matched_door: Option<(usize, usize, usize)> = None; // (door_index, segment_x, segment_z)
                                'find_door: for segment in &room.segments {
                                    for neighbour in segment.neighbours.iter().flatten() {
                                        if let Some(door) = self.doors.get(neighbour.door_index) {
                                            if door.door_type == wanted_door_type && !door.key_granted {
                                                matched_door = Some((neighbour.door_index, segment.x, segment.z));
                                                break 'find_door;
                                            }
                                        }
                                    }
                                }
                                if let Some((door_index, segment_x, segment_z)) = matched_door {
                                    self.doors[door_index].key_granted = true;
                                    // Center of the matched 32x32 room segment, not the door's own
                                    // coordinates - spawning at the door's position lands the key
                                    // literally inside the door frame (confirmed by testing).
                                    let spawn_pos = DVec3::new(
                                        DUNGEON_ORIGIN.0 as f64 + segment_x as f64 * 32.0 + 16.0,
                                        70.0,
                                        DUNGEON_ORIGIN.1 as f64 + segment_z as f64 * 32.0 + 16.0,
                                    );
                                    spawn_pickup(&mut server.world, spawn_pos, kind);
                                    let tnt_pos = DVec3::new(spawn_pos.x + 1.0, spawn_pos.y, spawn_pos.z);
                                    spawn_pickup(&mut server.world, tnt_pos, PickupKind::Tnt);
                                }
                            }
                        }
                    }
                }

                // A single flame particle at each of this room's locked chests, right at their
                // front face - locked chests are all placed once at world population, before
                // any player has connected, so a particle fired at placement time would never
                // have an observer. Firing it here instead, on room discovery, is the actual
                // moment a player could plausibly be watching.
                for &room_index in &rooms_just_entered {
                    // Inlined `get_room_at` - can't call it as a method here, `current_ticks`
                    // above already holds `self.state` mutably borrowed for this whole match
                    // arm, and a method call borrows all of `self`, not just `self.room_grid`.
                    let room_grid = &self.room_grid;
                    let chest_positions: Vec<BlockPos> = self.locked_chests.keys()
                        .filter(|&&pos| {
                            if pos.x < DUNGEON_ORIGIN.0 || pos.z < DUNGEON_ORIGIN.1 {
                                return false;
                            }
                            let grid_x = ((pos.x - DUNGEON_ORIGIN.0) / 32) as usize;
                            let grid_z = ((pos.z - DUNGEON_ORIGIN.1) / 32) as usize;
                            room_grid.get(grid_x + (grid_z * 6)).and_then(|e| *e) == Some(room_index)
                        })
                        .copied()
                        .collect();
                    for pos in chest_positions {
                        let direction = match server.world.get_block_at(pos.x, pos.y, pos.z) {
                            crate::server::block::blocks::Blocks::Chest { direction } => direction,
                            _ => continue,
                        };
                        let (fx, _, fz) = direction.get_offset();
                        let particle = crate::net::protocol::play::clientbound::Particles {
                            particle_id: crate::server::utils::particles::ParticleTypes::Flame.get_id(),
                            long_distance: false,
                            x: pos.x as f32 + 0.5 + fx as f32 * 0.6,
                            y: pos.y as f32 + 0.5,
                            z: pos.z as f32 + 0.5 + fz as f32 * 0.6,
                            offset_x: 0.0,
                            offset_y: 0.0,
                            offset_z: 0.0,
                            speed: 0.0,
                            count: 1,
                        };
                        for player in server.world.players.values_mut() {
                            player.write_packet(&particle);
                        }
                    }
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
                                        icons: vec![],
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
                let mut rooms_to_update_map: Vec<usize> = Vec::new();
                for (room_index, count) in room_secrets_found.into_iter() {
                    let count_u8: u8 = count; // Explicitly type as u8
                    let room = self.rooms.get_mut(room_index).unwrap();
                    let current: u16 = room.found_secrets as u16;
                    let add: u16 = count_u8 as u16;
                    let new_count: u8 = current.saturating_add(add).min(255) as u8;
                    let old_count = room.found_secrets;
                    room.found_secrets = new_count;
                    
                    // Track rooms that need map update (if secret count changed and room is entered)
                    if old_count != new_count && room.entered {
                        rooms_to_update_map.push(room_index);
                    }
                }
                
                // Redraw rooms on map if secrets changed
                for room_index in rooms_to_update_map {
                    self.map.draw_room(&self.rooms, &self.doors, room_index);
                    
                    // Send map update to all players
                    if let Some((region, data)) = self.map.get_updated_area() {
                        let width = region.max_x - region.min_x;
                        let height = region.max_y - region.min_y;
                        
                        for (_, player) in &mut server.world.players {
                            player.write_packet(&Maps {
                                id: 1,
                                scale: 0,
                                icons: vec![],
                                columns: width as u8,
                                rows: height as u8,
                                x: region.min_x as u8,
                                z: region.min_y as u8,
                                map_data: data.clone(),
                            });
                        }
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
            
            // Explode crypts (all patterns within radius) - a Crypt Undead spawns at each one's
            // position instead of the bonus score being granted immediately; the score is only
            // credited once that specific mob is killed (see `ai/combat.rs::kill_mob`), not just
            // for blowing the crypt block itself.
            let crypt_spawn_positions = room.explode_crypt_near(world, &pos, radius);

            // Explode King Midas's golden "crypt" the same way, but never inserted into
            // `entity_crypt_room` below - his kill is never credited as a real crypt (see
            // `Room::explode_kingmidas_near`).
            let kingmidas_spawn_positions = room.explode_kingmidas_near(world, &pos, radius);

            // Explode superboomwalls (only first pattern within radius)
            let walls_exploded = room.explode_superboomwall_near(world, &pos, radius);

            if !crypt_spawn_positions.is_empty() || !kingmidas_spawn_positions.is_empty() || walls_exploded > 0 {
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

            for crypt_pos in crypt_spawn_positions {
                let spawn_pos = DVec3::from(&crypt_pos).add_x(0.5).add_z(0.5);
                if let Some(entity_id) = crate::server::entity::dungeon_mobs::spawner::spawn_crypt_undead(world, room_index, spawn_pos, 0.0) {
                    world.entity_crypt_room.insert(entity_id, room_index);
                }
            }

            // King Midas spawns the same way, but deliberately isn't registered in
            // `entity_crypt_room` - see the doc comment above.
            for kingmidas_pos in kingmidas_spawn_positions {
                let spawn_pos = DVec3::from(&kingmidas_pos).add_x(0.5).add_z(0.5);
                crate::server::entity::dungeon_mobs::spawner::spawn_king_midas(world, room_index, spawn_pos, 0.0);
            }
        }
        Ok(())
    }

    // The four events below have no real trigger anywhere in this codebase yet - there's no
    // player damage/death system and no puzzle minigame implementation, so nothing calls
    // these automatically. They're exposed for whenever those systems exist (and for the
    // `/dscore` debug command, so the score/announcement logic itself can be exercised without
    // them).

    pub fn record_death(&mut self) {
        self.score.deaths += 1;
        let world = &mut self.server_mut().world;
        self.score.check_score_announcements(world);
    }

    pub fn record_puzzle_failed(&mut self) {
        self.score.failed_puzzles += 1;
        let world = &mut self.server_mut().world;
        self.score.check_score_announcements(world);
    }

    pub fn record_mimic_killed(&mut self) {
        self.score.mimic_killed = true;
        let world = &mut self.server_mut().world;
        self.score.check_score_announcements(world);
    }

    /// Credits a crypt's bonus score (+1, capped at +5 - see `bonus_score`) once its Crypt
    /// Undead is actually killed, not when the crypt block itself was exploded - see
    /// `superboom_at` (which spawns the mob instead of crediting immediately) and
    /// `ai/combat.rs::kill_mob` (which calls this on that mob's death).
    pub fn record_crypt_killed(&mut self) {
        self.score.crypts += 1;
        let world = &mut self.server_mut().world;
        self.score.check_score_announcements(world);
    }

    pub fn set_paul_ezpz(&mut self, enabled: bool) {
        self.score.paul_ezpz = enabled;
        let world = &mut self.server_mut().world;
        self.score.check_score_announcements(world);
    }
}


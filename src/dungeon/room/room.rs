use crate::dungeon::crushers::Crusher;
use crate::dungeon::door::Door;
use crate::dungeon::dungeon::DUNGEON_ORIGIN;
use crate::dungeon::room::room_data::{RoomData, RoomShape, RoomType};
use crate::dungeon::room::crypts::{get_room_crypts, get_room_kingmidas, rotate_block_pos};
use crate::dungeon::room::mushroom::{get_room_mushrooms, MushroomSets};
use crate::dungeon::room::superboomwalls::{get_room_superboomwalls, SuperboomWallPattern, rotate_superboomwall_pos};
use crate::dungeon::room::fallingblocks::{get_room_fallingblocks, FallingBlockPattern, rotate_fallingblock_pos};
use crate::dungeon::room::levers::{get_room_levers, LeverData};
use crate::dungeon::room::locked_chests::{get_room_locked_chests, facing_string_to_direction};
use crate::dungeon::dungeon::{Dungeon, LockedChestState};
use crate::utils::seeded_rng::seeded_rng;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::utils::direction::Direction;
use crate::server::world::World;
use std::collections::HashSet;
use rand::Rng;

#[derive(Debug)]
pub struct RoomSegment {
    pub x: usize,
    pub z: usize,
    pub neighbours: [Option<RoomNeighbour>; 4]
}

#[derive(Debug, Clone, Copy)]
pub struct RoomNeighbour {
    pub door_index: usize,
    pub room_index: usize,
}

#[derive(Debug)]
pub struct Room {
    pub segments: Vec<RoomSegment>,
    pub room_data: RoomData,
    pub rotation: Direction,

    pub tick_amount: u32,
    pub crushers: Vec<Crusher>,
    pub crypt_patterns: Vec<Vec<(BlockPos, Option<u16>)>>, // world positions with expected block ids
    pub crypts_checked: bool,
    pub crypts_detected_count: usize,
    /// King Midas's golden "crypt" - same explode-on-superboom shape as a real crypt, kept in
    /// its own list so it's never touched by `explode_crypt_near`/counted by `detect_crypts`.
    /// See `crypts::get_room_kingmidas` and `explode_kingmidas_near`.
    pub kingmidas_patterns: Vec<Vec<(BlockPos, Option<u16>)>>,
    pub superboomwall_patterns: Vec<SuperboomWallPattern>, // superboomwall patterns for this room
    pub superboomwalls_checked: bool,
    pub superboomwalls_detected_count: usize,
    pub fallingblock_patterns: Vec<FallingBlockPattern>, // falling block patterns for this room
    pub fallingblocks_checked: bool,
    pub fallingblocks_detected_count: usize,
    pub scheduled_falling_removals: Vec<(u64, Vec<crate::dungeon::room::fallingblocks::FallingBlock>)>, // (tick, blocks)
    /// Indices into `fallingblock_patterns` that have been claimed by a scheduled drop - inserted
    /// the moment a pattern is scheduled and never removed again (not even once it actually
    /// drops), so it can never be found or scheduled a second time. Doubles as both "don't
    /// re-arm while this is waiting on the shared 5-tick pulse" and, after it fires, "already
    /// dropped" - replacing what used to be an actual removal from `fallingblock_patterns` at
    /// trigger time, which isn't safe now that more than one pattern can be waiting on its own
    /// scheduled tick simultaneously (removing by index would shift every other still-pending
    /// index). See `check_fallingblocks_collision`'s doc comment for the timing rule itself.
    pub pending_fallingblock_triggers: std::collections::HashSet<usize>,
    pub mushroom_sets: Vec<MushroomSets>,
    pub lever_data: Vec<LeverData>, // Store lever data for this room
    
    pub entered: bool,
    pub found_secrets: u8, // Number of secrets found in this room (runtime tracking)
    pub json_secrets: Vec<std::rc::Rc<std::cell::RefCell<crate::dungeon::room::secrets::DungeonSecret>>>, // Secrets from secrets.json
    pub room_entry_secrets_spawned: bool, // Track if schest/sess have been spawned on room entry

    /// Number of starred dungeon mobs spawned into this room that haven't died yet - the map
    /// only shows a checkmark once this hits 0 (rooms with no starred mobs start at 0, i.e.
    /// count as cleared immediately). See `spawner.rs` (incremented on spawn) and
    /// `ai/combat.rs::kill_mob` (decremented on death, triggers a map redraw at 0).
    pub starred_mobs_remaining: u32,

    /// Whether `spawn_room_mobs` has actually run for this room yet. Room entry and mob spawning
    /// are no longer the same instant - entry queues the room behind `Dungeon`'s mob-spawn cycle
    /// (see `MOB_SPAWN_CYCLE_TICKS`), so `starred_mobs_remaining` reads 0 for a room that simply
    /// hasn't had its mobs spawned in yet, same as a room with no starred mobs at all. Without
    /// this, `DungeonMap::draw_room`'s checkmark logic couldn't tell those two 0s apart and drew
    /// a checkmark the instant the room was entered, before any of its mobs even existed.
    pub mobs_spawned: bool,

    /// Whether this room has already been queued for a dormant pre-spawn (see
    /// `Dungeon::tick`'s adjacent-room check) - a one-shot latch set the instant it's queued
    /// (not when `spawn_room_mobs` actually runs, same timing `entered` itself already uses for
    /// `rooms_just_entered`), so a player lingering in a neighbouring room for many ticks before
    /// the mob-spawn cycle fires doesn't push this room's index into `pending_mob_spawn_rooms`
    /// over and over. Also true once the room is entered directly (`mobs_spawned` covers that
    /// case identically), so this only ever matters for the adjacent-room path.
    pub mob_prespawn_queued: bool,

    /// Whether `three_weirdos::setup` has already run for this room instance. The 3 NPCs used to
    /// spawn unconditionally at room load (`load_into_world`) - now deferred to the same
    /// room-entry hook `room_entry_secrets_spawned`/`mobs_spawned` use (`Dungeon::tick`'s
    /// `rooms_just_entered` set), so a player never sees them pop in before actually crossing
    /// into the room. This flag is what makes that idempotent - walking out and back in must not
    /// re-roll the names or spawn duplicates.
    pub weirdos_spawned: bool,

    /// Whether `quiz::setup` has already run for this room instance - same idempotency role as
    /// `weirdos_spawned`, guarding the room-entry hook (`Dungeon::tick`'s `rooms_just_entered`
    /// set) against re-firing Oruo's intro speech if a player walks out and back in.
    pub quiz_started: bool,

    /// Whether `ice_path::setup` has already run for this room instance - same idempotency role
    /// as `quiz_started`, guarding the room-entry hook against re-spawning the silverfish if a
    /// player walks out and back in.
    pub ice_path_spawned: bool,

    /// Whether `waterboard::setup` has already run for this room instance - same idempotency
    /// role as `ice_path_spawned`. Per explicit confirmation, the 3 randomly-closed gates'
    /// pistons physically push shut (with a real piston sound) the moment a player actually
    /// crosses into the room, not silently at dungeon generation before anyone's there to see or
    /// hear it - same room-entry hook as Three Weirdos/Quiz/Ice Path above, guarded so walking
    /// out and back in doesn't re-roll which gates are closed or re-register the levers.
    pub waterboard_spawned: bool,

    /// Whether `shadow_assassin::setup` has already run for this room instance - same
    /// idempotency role as `waterboard_spawned`, guarding the room-entry hook against
    /// re-spawning the miniboss if a player walks out and back in.
    pub shadow_assassin_spawned: bool,

    /// Whether `default_room::setup` has already run for this room instance - same idempotency
    /// role as `shadow_assassin_spawned`, guarding the room-entry hook against re-spawning the
    /// guaranteed Lost Adventurer if a player walks out and back in.
    pub default_lost_adventurer_spawned: bool,

    /// Whether `dragon::setup` has already run for this room instance - same idempotency role
    /// as `default_lost_adventurer_spawned`, guarding the room-entry hook against re-rolling/
    /// re-spawning the random miniboss if a player walks out and back in.
    pub dragon_miniboss_spawned: bool,

    /// Whether `default_dirt::setup` has already run for this room instance - same idempotency
    /// role as `dragon_miniboss_spawned`, for the *other* "Default" room capture (see
    /// `default_dirt.rs`'s own doc comment).
    pub default_dirt_miniboss_spawned: bool,

    /// World position of the one specific secret chest that marks a Trap room "cleared" - `None`
    /// for every room except the two known Trap layouts (see `trap_completion_secret_relative_pos`).
    /// Trap rooms have no starred mobs at all, so the ordinary `starred_mobs_remaining == 0`
    /// check `DungeonMap::draw_room` uses for the checkmark reads true the instant the room is
    /// entered - real Hypixel instead ties a Trap room's clear state to grabbing this specific
    /// chest (confirmed against the real in-room coordinates: Old Trap's is at relative (4, 71,
    /// 9), New Trap's at (26, 90, 14) - both taken from `secrets_loader.rs`'s own `schest` data
    /// for those rooms, not guessed).
    pub trap_completion_chest_pos: Option<BlockPos>,
    /// Whether `trap_completion_chest_pos`'s secret has been obtained - see
    /// `block_interact_action.rs`'s `Chest` handler (sets this) and `DungeonMap::draw_room`
    /// (reads it in place of `starred_mobs_remaining == 0` for Trap rooms).
    pub trap_completed: bool,

    /// Whether this room's puzzle has been resolved (solved *or* failed - either way it's done,
    /// matching real Hypixel: a failed puzzle still marks the room complete on the map, it just
    /// costs score). `false` for every non-`RoomType::Puzzle` room, which never sets it. Same
    /// role for puzzle rooms that `trap_completed` plays for Trap rooms - see that field's doc
    /// comment and `DungeonMap::draw_room`'s `is_cleared` check, which reads this instead of the
    /// usual `starred_mobs_remaining == 0` (puzzle rooms have no starred mobs, so that check
    /// would otherwise read true the instant the room is entered).
    pub puzzle_completed: bool,

    /// Set alongside `puzzle_completed` whenever a puzzle resolves as a *failure* specifically
    /// (out-of-order Blaze kill, wrong Three Weirdos chest, wrong Quiz answer, a Tic Tac Toe
    /// loss, etc.) - `puzzle_completed` alone can't distinguish solved from failed (it's true
    /// for both, matching real Hypixel's map checkmark - see that field's doc comment), but the
    /// tab-list puzzle checklist Odin reads genuinely does distinguish the two outcomes with a
    /// separate glyph (`DungeonListener.kt`'s `puzzleRegex`: "✔" -> Completed, "✖" -> Failed) -
    /// this is what `main.rs`'s tab-list line generation checks to pick between them. `false`
    /// for every non-`RoomType::Puzzle` room and for any puzzle that hasn't failed.
    pub puzzle_failed: bool,

    /// `Some` only for the one room (if any) whose `room_data.name` is "Teleport Maze" - set by
    /// `teleport_maze::setup`. Lives directly on `Room` rather than in a `world.interactable_blocks`
    /// entry like `three_weirdos`/`creeper_beams`'s puzzle state, because a teleport pad triggers
    /// by being walked onto (checked every tick in `teleport_maze::tick`, called from `Room::tick`
    /// below), not right-clicked - there's no single interactable block position to hang an
    /// `Rc` off of instead.
    pub teleport_maze_state: Option<std::rc::Rc<std::cell::RefCell<crate::dungeon::room::teleport_maze::TeleportMazeState>>>,

    /// `Some` only for the one room (if any) whose `room_data.name` is "Tic Tac Toe" - set by
    /// `tic_tac_toe::setup`. Currently only backs the board's display (the 9 Item Frames'
    /// entity ids and each cell's shown state) - the actual game (buttons, AI, win detection)
    /// isn't wired up yet, but the state already lives here, same `Rc<RefCell<>>` pattern as
    /// every other puzzle, so that follow-up has something to reach into.
    pub tic_tac_toe_state: Option<std::rc::Rc<std::cell::RefCell<crate::dungeon::room::tic_tac_toe::TicTacToeState>>>,

    /// `Some` only for the one room (if any) whose `room_data.name` is "Ice Fill" - set by
    /// `ice_fill::setup`. Same reasoning/timing as `teleport_maze_state` - an ice tile triggers by
    /// being walked onto (checked every tick in `ice_fill::tick`, called from `Room::tick` below),
    /// not right-clicked.
    pub ice_fill_state: Option<std::rc::Rc<std::cell::RefCell<crate::dungeon::room::ice_fill::IceFillState>>>,

    /// `Some` only for the one room (if any) whose `room_data.name` is "Boulder" - set by
    /// `boulder::setup`. A box's own trigger positions relocate on every push, so unlike a fixed-
    /// position puzzle (Tic Tac Toe's buttons, Creeper Beams' lanterns) this needs its own state
    /// reachable independent of any one `BlockInteractAction` - see `boulder::BoulderState`'s own
    /// doc comment.
    pub boulder_state: Option<std::rc::Rc<std::cell::RefCell<crate::dungeon::room::boulder::BoulderState>>>,

    /// `Some` only for the one room (if any) whose `room_data.name` is "Water Board" - set by
    /// `waterboard::setup`. Reachable independent of any one lever's own `BlockInteractAction` so
    /// `waterboard::tick` (called from `Room::tick` below) can advance the room-scoped water flow
    /// simulation every tick while the WATER lever is on, same reasoning `boulder_state` needed.
    pub waterboard_state: Option<std::rc::Rc<std::cell::RefCell<crate::dungeon::room::waterboard::WaterboardState>>>,

    /// `Some` only for the one room (if any) whose `room_data.name` is "Blaze" (real names "Lower
    /// Blaze"/"Higher Blaze") - set by `blaze::setup`. Reachable by `room_index` from each
    /// spawned Blaze's own `EntityImpl::interact` (an entity has no direct path back to its own
    /// `Room`), the same reasoning `boulder_state` needed for its triggers.
    pub blaze_state: Option<std::rc::Rc<std::cell::RefCell<crate::dungeon::room::blaze::BlazeState>>>,

    /// `Some` only for the one room (if any) whose `room_data.name` is "Blaze" AND whose variant
    /// (`bottom`) has a captured reward-chest shaft - set by `blaze::setup`. Reachable by
    /// `room_index`/ticked every room tick the same way `boulder_state` is, since the chest's
    /// animation runs on a tick timer, not directly off any one interact/kill event.
    pub blaze_chest_state: Option<std::rc::Rc<std::cell::RefCell<crate::dungeon::room::blaze::ChestShaftState>>>,
}

/// Relative (pre-rotation) position of the specific secret chest whose pickup marks a Trap room
/// "cleared" - see `Room::trap_completion_chest_pos`. `None` for anything that isn't a known trap
/// layout (including rooms not yet scraped/added).
fn trap_completion_secret_relative_pos(room_name: &str) -> Option<BlockPos> {
    match room_name {
        "Old Trap" => Some(BlockPos { x: 4, y: 71, z: 9 }),
        "New Trap" => Some(BlockPos { x: 26, y: 90, z: 14 }),
        _ => None,
    }
}

/// Extra rotation to apply on top of `Room::get_rotation_from_segments`'s own computed value, for
/// the rare room whose captured door doesn't actually sit on this project's usual canonical local
/// side. Confirmed real (per explicit user report): "Dragon" (a 1x1 dead-end/`OneByOneEnd` room)
/// was always ending up rotated 180° wrong - its door came out of the back of the room instead of
/// the front, meaning its own captured `block_data` has the real door on the opposite local side
/// from what `get_1x1_shape_and_type`'s dead-end rotation table assumes. Rather than transform the
/// captured block data itself, this corrects it at the one place `rotation` is computed for every
/// room, fixing the door alignment and every other rotated element (facing, secrets, crushers,
/// etc.) consistently in one place, same as the rest of this function.
fn door_rotation_correction(room_name: &str) -> Option<Direction> {
    match room_name {
        "Dragon" => Some(Direction::South), // Direction::South = a 180° flip, see `Rotatable`.
        _ => None,
    }
}

impl Room {

    pub fn new(
        mut segments: Vec<RoomSegment>,
        dungeon_doors: &[Door],
        room_data: RoomData
    ) -> Room {
        // Sort room segments by z and then x
        segments.sort_by(|a, b| a.z.cmp(&b.z));
        segments.sort_by(|a, b| a.x.cmp(&b.x));
        
        let mut rotation = Room::get_rotation_from_segments(&segments, dungeon_doors);
        if let Some(correction) = door_rotation_correction(&room_data.name) {
            rotation = rotation.rotate(correction);
        }
        let corner_pos = Room::get_corner_pos_from(&segments, &rotation, &room_data);

        // Same relative -> world transform `secrets_loader.rs` uses for `schest` entries (Y is
        // absolute, only X/Z rotate) - has to match exactly, since this is compared directly
        // against a loaded secret's own `block_pos` later (see `trap_completion_chest_pos`).
        let trap_completion_chest_pos = trap_completion_secret_relative_pos(&room_data.name).map(|rel| {
            let rotated = rel.rotate(rotation);
            BlockPos { x: corner_pos.x + rotated.x, y: rotated.y, z: corner_pos.z + rotated.z }
        });

        let crushers = room_data.crusher_data.iter().map(|data| {
            let mut crusher = Crusher::from_json(data);
            
            crusher.direction = crusher.direction.rotate(rotation);
            crusher.block_pos = crusher.block_pos.rotate(rotation);

            // This is fucking aids
            match rotation {
                Direction::North => match crusher.direction {
                    Direction::East | Direction::West => crusher.block_pos.add_z(crusher.width - 1),
                    _ => crusher.block_pos.add_x(crusher.width - 1),
                },
                Direction::South => match crusher.direction {
                    Direction::East | Direction::West => crusher.block_pos.add_z(-crusher.width + 1),
                    _ => crusher.block_pos.add_x(-crusher.width + 1),
                }
                _ => crusher.block_pos,
            };

            crusher.block_pos = crusher.block_pos
                .add_x(corner_pos.x)
                .add_z(corner_pos.z);

            crusher
        }).collect::<Vec<Crusher>>();

        // Build crypt patterns from relative coords json
        let mut crypt_patterns: Vec<Vec<(BlockPos, Option<u16>)>> = Vec::new();

        let shape_key = match room_data.shape {
            RoomShape::OneByOne => "1x1",
            RoomShape::OneByOneEnd => "1x1_E",
            RoomShape::OneByOneCross => "1x1_X",
            RoomShape::OneByOneStraight => "1x1_I",
            RoomShape::OneByOneBend => "1x1_L",
            RoomShape::OneByOneTriple => "1x1_3",
            RoomShape::OneByTwo => "1x2",
            _ => "rest",
        };

        if let Some(rc) = get_room_crypts(shape_key, &room_data.name) {
            for pattern in rc.patterns {
                let mut world_blocks: Vec<(BlockPos, Option<u16>)> = Vec::new();
                for blk in pattern.blocks {
                    let rotated = rotate_block_pos(&blk, rotation);
                    // Room block data is placed at its original absolute Y levels,
                    // so crypt coordinates use their absolute Y directly.
                    let world_pos = BlockPos { x: corner_pos.x + rotated.x, y: blk.y, z: corner_pos.z + rotated.z };
                    world_blocks.push((world_pos, blk.block_id));
                }
                crypt_patterns.push(world_blocks);
            }
        }

        // Build King Midas's golden "crypt" pattern from its own relative-coords json - same
        // shape/rotation handling as a real crypt above, but tracked separately so it's never
        // credited as one (see `crypts::get_room_kingmidas`).
        let mut kingmidas_patterns: Vec<Vec<(BlockPos, Option<u16>)>> = Vec::new();
        if let Some(rc) = get_room_kingmidas(&room_data.name) {
            for pattern in rc.patterns {
                let mut world_blocks: Vec<(BlockPos, Option<u16>)> = Vec::new();
                for blk in pattern.blocks {
                    let rotated = rotate_block_pos(&blk, rotation);
                    let world_pos = BlockPos { x: corner_pos.x + rotated.x, y: blk.y, z: corner_pos.z + rotated.z };
                    world_blocks.push((world_pos, blk.block_id));
                }
                kingmidas_patterns.push(world_blocks);
            }
        }

        // Build mushroom secret sets
        let mushroom_sets = get_room_mushrooms(&room_data.name, rotation, &corner_pos);

        // Build superboomwall patterns from relative coords json
        let mut superboomwall_patterns = Vec::new();
        if let Some(patterns) = get_room_superboomwalls(shape_key, &room_data.name) {
            for pattern in patterns {
                let mut world_blocks = Vec::new();
                for block in pattern.blocks {
                    // Convert relative coordinates to world coordinates
                    let rotated = rotate_superboomwall_pos(&block, rotation);
                    let world_pos = BlockPos { 
                        x: corner_pos.x + rotated.x, 
                        y: block.y, // Use absolute Y like crypts
                        z: corner_pos.z + rotated.z 
                    };
                    world_blocks.push(crate::dungeon::room::superboomwalls::SuperboomWallBlock {
                        x: world_pos.x,
                        y: world_pos.y,
                        z: world_pos.z,
                        block_id: block.block_id,
                    });
                }
                superboomwall_patterns.push(crate::dungeon::room::superboomwalls::SuperboomWallPattern {
                    blocks: world_blocks,
                });
            }
        }

        // Build falling block patterns from relative coords json
        let mut fallingblock_patterns = Vec::new();
        if let Some(patterns) = get_room_fallingblocks(shape_key, &room_data.name) {
            for pattern in patterns {
                let mut world_blocks = Vec::new();
                for block in pattern.blocks {
                    // Convert relative coordinates to world coordinates
                    let rotated = rotate_fallingblock_pos(&block, rotation);
                    let world_pos = BlockPos { 
                        x: corner_pos.x + rotated.x, 
                        y: block.y, // Use absolute Y like crypts
                        z: corner_pos.z + rotated.z 
                    };
                    world_blocks.push(crate::dungeon::room::fallingblocks::FallingBlock {
                        x: world_pos.x,
                        y: world_pos.y,
                        z: world_pos.z,
                        block_id: block.block_id,
                    });
                }
                fallingblock_patterns.push(crate::dungeon::room::fallingblocks::FallingBlockPattern {
                    blocks: world_blocks,
                });
            }
        }

        // Store lever data for this room (similar to crypts and superboom walls)
        let mut lever_data = Vec::new();
        if let Some(room_levers) = get_room_levers(shape_key, &room_data.name) {
            for lever_data_item in room_levers {
                // Convert relative coordinates to world coordinates (same as crypts)
                let relative_pos = BlockPos {
                    x: lever_data_item.lever[0],
                    y: lever_data_item.lever[1], // Y coordinates are absolute
                    z: lever_data_item.lever[2],
                };
                
                // Rotate the position based on room rotation (same as crypts)
                let rotated = relative_pos.rotate(rotation);
                
                // Convert to world coordinates (same as crypts)
                let lever_pos = BlockPos {
                    x: corner_pos.x + rotated.x,
                    y: rotated.y, // Y coordinates are absolute
                    z: corner_pos.z + rotated.z,
                };
                
                // Store the lever data with world coordinates
                let mut world_lever_data = lever_data_item.clone();
                world_lever_data.lever = [lever_pos.x, lever_pos.y, lever_pos.z];
                
                // Convert block positions to world coordinates as well
                let mut world_blocks = Vec::new();
                for block_pos in &lever_data_item.blocks {
                    let relative_block_pos = BlockPos {
                        x: block_pos[0],
                        y: block_pos[1], // Y coordinates are absolute
                        z: block_pos[2],
                    };
                    
                    // Rotate the block position based on room rotation (same as crypts)
                    let rotated_block_pos = relative_block_pos.rotate(rotation);
                    
                    // Convert to world coordinates (same as crypts)
                    let world_block_pos = BlockPos {
                        x: corner_pos.x + rotated_block_pos.x,
                        y: rotated_block_pos.y, // Y coordinates are absolute
                        z: corner_pos.z + rotated_block_pos.z,
                    };
                    
                    world_blocks.push([world_block_pos.x, world_block_pos.y, world_block_pos.z]);
                }
                world_lever_data.blocks = world_blocks;
                
                lever_data.push(world_lever_data);
            }
        }

        Room {
            segments,
            room_data,
            rotation,
            tick_amount: 0,
            crushers,
            crypt_patterns,
            crypts_checked: false,
            crypts_detected_count: 0,
            kingmidas_patterns,
            superboomwall_patterns,
            superboomwalls_checked: false,
            superboomwalls_detected_count: 0,
            fallingblock_patterns,
            fallingblocks_checked: false,
            fallingblocks_detected_count: 0,
            scheduled_falling_removals: Vec::new(),
            pending_fallingblock_triggers: std::collections::HashSet::new(),
            mushroom_sets,
            lever_data,
            entered: false,
            found_secrets: 0,
            json_secrets: Vec::new(),
            room_entry_secrets_spawned: false,
            starred_mobs_remaining: 0,
            mobs_spawned: false,
            mob_prespawn_queued: false,
            weirdos_spawned: false,
            quiz_started: false,
            ice_path_spawned: false,
            waterboard_spawned: false,
            shadow_assassin_spawned: false,
            default_lost_adventurer_spawned: false,
            dragon_miniboss_spawned: false,
            default_dirt_miniboss_spawned: false,
            trap_completion_chest_pos,
            trap_completed: false,
            puzzle_completed: false,
            puzzle_failed: false,
            teleport_maze_state: None,
            tic_tac_toe_state: None,
            ice_fill_state: None,
            boulder_state: None,
            waterboard_state: None,
            blaze_state: None,
            blaze_chest_state: None,
        }
    }

    pub fn get_corner_pos(&self) -> BlockPos {
        Room::get_corner_pos_from(&self.segments, &self.rotation, &self.room_data)
    }

    /// Every real door-connected neighbour room's index, across all of this room's own segments
    /// (`RoomSegment::neighbours`, precomputed once in `Dungeon::from_layout`) - used to find
    /// which rooms should get their mobs dormant-pre-spawned the instant a player enters an
    /// adjacent room connected by a door (see `Dungeon::tick`'s own doc comment on that check).
    /// May yield the same room index more than once if two of this room's own segments both
    /// border it - callers already guard against re-queuing via `mob_prespawn_queued`, so this
    /// doesn't need to deduplicate itself.
    pub fn neighbouring_room_indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.segments.iter()
            .flat_map(|segment| segment.neighbours.iter())
            .filter_map(|neighbour| neighbour.map(|n| n.room_index))
    }

    pub fn get_corner_pos_from(segments: &[RoomSegment], rotation: &Direction, room_data: &RoomData) -> BlockPos {
        let min_x = segments.iter().min_by(|a, b| a.x.cmp(&b.x)).unwrap().x;
        let min_z = segments.iter().min_by(|a, b| a.z.cmp(&b.z)).unwrap().z;

        // Special handling for bossrooms - commented out
        // if room_data.room_type == crate::dungeon::room::room_data::RoomType::Boss {
        //     match rotation {
        //         Direction::North => BlockPos { x: -8, y: 254, z: -8 },
        //         Direction::East => BlockPos { x: -8 + room_data.length - 1, y: 254, z: -8 },
        //         Direction::South => BlockPos { x: -8 + room_data.length - 1, y: 254, z: -8 + room_data.width - 1 },
        //         Direction::West => BlockPos { x: -8, y: 254, z: -8 + room_data.width - 1 },
        //         _ => unreachable!(),
        //     }
        // } else {
        let x = min_x as i32 * 32 + DUNGEON_ORIGIN.0;
        let y = 68;
        let z = min_z as i32 * 32 + DUNGEON_ORIGIN.1;
        
        match rotation {
            Direction::North => BlockPos { x, y, z },
            Direction::East => BlockPos { x: x + room_data.length - 1, y, z },
            Direction::South => BlockPos { x: x + room_data.length - 1, y, z: z + room_data.width - 1 },
            Direction::West => BlockPos { x, y, z: z + room_data.width - 1 },
            _ => unreachable!(),
        }
        // }
    }

    pub fn tick(&mut self, world: &mut World) {
        self.tick_amount += 1;

        // Process scheduled falling block removals
        self.process_scheduled_falling_removals(world);

        // Checks every player's position against the Teleport Maze's pad list, if this room has
        // one - no-op otherwise (see `teleport_maze_state`'s doc comment for why this needs a
        // per-tick check instead of the click-based `BlockInteractAction` every other puzzle uses).
        crate::dungeon::room::teleport_maze::tick(self, world);

        // Same per-tick position check as Teleport Maze above, for Ice Fill's own walked-not-
        // clicked ice tiles - no-op for every room but the one with an active `ice_fill_state`.
        crate::dungeon::room::ice_fill::tick(self, world);

        // Boulder's floor light-up effect (barrier <-> white stained glass near a player's feet)
        // - no-op for every room but the one with an active `boulder_state`.
        crate::dungeon::room::boulder::tick(self, world);

        // Higher/Lower Blaze's reward-chest shaft animation (one block step every
        // `blaze::CHEST_STEP_TICKS` ticks while a move is pending) - no-op for every room but
        // the one with an active `blaze_chest_state`.
        crate::dungeon::room::blaze::tick(self, world);

        // Water Board's own room-scoped water flow simulation (see `waterboard::tick`'s doc
        // comment) - no-op for every room but the one with an active `waterboard_state` whose
        // WATER lever is currently on.
        crate::dungeon::room::waterboard::tick(self, world);

        // Places the Tic Tac Toe bot's own already-computed move once its 3-second delay
        // actually elapses (see `tic_tac_toe::tick`'s doc comment) - no-op for every room but
        // the one with an active `tic_tac_toe_state` that currently has a move pending.
        crate::dungeon::room::tic_tac_toe::tick(self, world);
    }

    pub fn detect_crypts(&mut self, world: &World) -> usize {
        if self.crypts_checked {
            return self.crypts_detected_count;
        }
        let mut detected = 0usize;
        'pattern: for pattern in &self.crypt_patterns {
            for (pos, expected) in pattern {
                if let Some(block_id) = expected {
                    let state_id = world.get_block_at(pos.x, pos.y, pos.z).get_block_state_id();
                    let id = (state_id >> 4) as u16;
                    if id != *block_id {
                        continue 'pattern;
                    }
                }
            }
            detected += 1;
        }
        self.crypts_detected_count = detected;
        self.crypts_checked = true;
        detected
    }

    /// Explodes (removes) all crypt patterns that have any block within `radius`
    /// of `center`. Returns one spawn position (the pattern's block centroid) per crypt
    /// exploded - used by `Dungeon::superboom_at` to spawn a Crypt Undead standing where each
    /// one was, instead of granting the crypt's bonus score immediately on explosion.
    pub fn explode_crypt_near(&mut self, world: &mut World, center: &BlockPos, radius: i32) -> Vec<BlockPos> {
        if self.crypt_patterns.is_empty() { return Vec::new(); }

        // Collect indices to remove to avoid borrow issues while mutating
        // Vertical reach is one block short of the horizontal radius - real Superboom TNT
        // doesn't blow out a full symmetric cube; using `radius` on Y too let it reach one
        // block further up/down than intended (e.g. one block below when placed up high).
        let vertical_radius = radius - 1;
        let mut indices: Vec<usize> = Vec::new();
        for (i, pattern) in self.crypt_patterns.iter().enumerate() {
            let in_range = pattern.iter().any(|(pos, _)| {
                let dx = (pos.x - center.x).abs();
                let dy = (pos.y - center.y).abs();
                let dz = (pos.z - center.z).abs();
                dx.max(dz) <= radius && dy <= vertical_radius
            });
            if in_range { indices.push(i); }
        }

        if indices.is_empty() { return Vec::new(); }

        // Remove from highest to lowest index to keep indices valid
        indices.sort_unstable_by(|a, b| b.cmp(a));
        let mut spawn_positions = Vec::new();
        for idx in indices {
            if let Some(pattern) = self.crypt_patterns.get(idx).cloned() {
                let count = pattern.len() as i32;
                let (sum_x, sum_y, sum_z) = pattern.iter()
                    .fold((0, 0, 0), |(sx, sy, sz), (pos, _)| (sx + pos.x, sy + pos.y, sz + pos.z));
                spawn_positions.push(BlockPos { x: sum_x / count, y: sum_y / count, z: sum_z / count });

                for (pos, _) in pattern.into_iter() {
                    world.set_block_at(Blocks::Air, pos.x, pos.y, pos.z);
                }
                // Actually remove the pattern after applying blocks
                let _ = self.crypt_patterns.remove(idx);
            }
        }

        spawn_positions
    }

    /// Explodes King Midas's golden "crypt" the same way `explode_crypt_near` does for a real
    /// one, but this is never wired to `entity_crypt_room`/`record_crypt_killed` - see
    /// `Dungeon::superboom_at`, which spawns the King Midas NPC from these positions instead of
    /// a Crypt Undead and never credits the crypt bonus score for it.
    pub fn explode_kingmidas_near(&mut self, world: &mut World, center: &BlockPos, radius: i32) -> Vec<BlockPos> {
        if self.kingmidas_patterns.is_empty() { return Vec::new(); }

        // See `explode_crypt_near`'s doc comment on `vertical_radius` for why Y is bounded a
        // block short of the horizontal radius rather than reusing it directly.
        let vertical_radius = radius - 1;
        let mut indices: Vec<usize> = Vec::new();
        for (i, pattern) in self.kingmidas_patterns.iter().enumerate() {
            let in_range = pattern.iter().any(|(pos, _)| {
                let dx = (pos.x - center.x).abs();
                let dy = (pos.y - center.y).abs();
                let dz = (pos.z - center.z).abs();
                dx.max(dz) <= radius && dy <= vertical_radius
            });
            if in_range { indices.push(i); }
        }

        if indices.is_empty() { return Vec::new(); }

        indices.sort_unstable_by(|a, b| b.cmp(a));
        let mut spawn_positions = Vec::new();
        for idx in indices {
            if let Some(pattern) = self.kingmidas_patterns.get(idx).cloned() {
                let count = pattern.len() as i32;
                let (sum_x, sum_y, sum_z) = pattern.iter()
                    .fold((0, 0, 0), |(sx, sy, sz), (pos, _)| (sx + pos.x, sy + pos.y, sz + pos.z));
                spawn_positions.push(BlockPos { x: sum_x / count, y: sum_y / count, z: sum_z / count });

                for (pos, _) in pattern.into_iter() {
                    world.set_block_at(Blocks::Air, pos.x, pos.y, pos.z);
                }
                let _ = self.kingmidas_patterns.remove(idx);
            }
        }

        spawn_positions
    }

    /// Detect superboomwalls in the room (similar to crypts)
    pub fn detect_superboomwalls(&mut self, world: &World) -> usize {
        if self.superboomwalls_checked {
            return self.superboomwalls_detected_count;
        }
        let mut detected = 0usize;
        'pattern: for pattern in &self.superboomwall_patterns {
            for block in &pattern.blocks {
                let pos = BlockPos { x: block.x, y: block.y, z: block.z };
                let state_id = world.get_block_at(pos.x, pos.y, pos.z).get_block_state_id();
                let id = (state_id >> 4) as u16;
                if id != block.block_id {
                    continue 'pattern;
                }
            }
            detected += 1;
        }
        self.superboomwalls_detected_count = detected;
        self.superboomwalls_checked = true;
        detected
    }

    /// Explodes (removes) ONLY THE FIRST superboomwall pattern that has any block within `radius`
    /// of `center`. Returns the number of walls exploded (0 or 1).
    /// This is the key difference from crypts - only one wall can be exploded at a time.
    pub fn explode_superboomwall_near(&mut self, world: &mut World, center: &BlockPos, radius: i32) -> usize {
        if self.superboomwall_patterns.is_empty() { 
            return 0; 
        }

        // See `explode_crypt_near`'s doc comment on `vertical_radius` for why Y is bounded a
        // block short of the horizontal radius rather than reusing it directly.
        let vertical_radius = radius - 1;

        // Find the FIRST pattern that has any block within range
        for (i, pattern) in self.superboomwall_patterns.iter().enumerate() {
            let in_range = pattern.blocks.iter().any(|block| {
                let pos = BlockPos { x: block.x, y: block.y, z: block.z };
                let dx = (pos.x - center.x).abs();
                let dy = (pos.y - center.y).abs();
                let dz = (pos.z - center.z).abs();
                dx.max(dz) <= radius && dy <= vertical_radius
            });
            
            if in_range {
                // Found the first wall in range - explode it and return
                if let Some(pattern) = self.superboomwall_patterns.get(i).cloned() {
                    for block in pattern.blocks {
                        let pos = BlockPos { x: block.x, y: block.y, z: block.z };
                        world.set_block_at(Blocks::Air, pos.x, pos.y, pos.z);
                    }
                    // Remove the pattern after applying blocks
                    let _ = self.superboomwall_patterns.remove(i);
                    return 1; // Only one wall exploded
                }
            }
        }

        0 // No walls in range
    }

    /// Detect falling blocks in the room (similar to crypts)
    pub fn detect_fallingblocks(&mut self, world: &World) -> usize {
        if self.fallingblocks_checked {
            return self.fallingblocks_detected_count;
        }
        let mut detected = 0usize;
        'pattern: for pattern in &self.fallingblock_patterns {
            for block in &pattern.blocks {
                let pos = BlockPos { x: block.x, y: block.y, z: block.z };
                let state_id = world.get_block_at(pos.x, pos.y, pos.z).get_block_state_id();
                let id = (state_id >> 4) as u16;
                if id != block.block_id {
                    continue 'pattern;
                }
            }
            detected += 1;
        }
        self.fallingblocks_detected_count = detected;
        self.fallingblocks_checked = true;
        detected
    }

    /// Checks if the player is standing on any (not-yet-triggered/not-yet-scheduled) falling
    /// block pattern, and if so, schedules it to drop exactly 5 ticks from the step itself,
    /// rather than dropping it immediately. Returns true if a pattern was newly scheduled this
    /// call.
    ///
    /// Each pattern gets its own fixed 5-tick timer starting the instant it's stepped on - not a
    /// clock shared across patterns. (An earlier version synced every pattern to a shared pulse
    /// derived from `world.tick_count % 5`, so the actual wait could be as little as 1 tick
    /// depending on when you happened to step on it - practically instant and not an actual
    /// 5-tick delay.) Once a pattern is scheduled, it commits - it fires at the scheduled tick
    /// regardless of whether anyone is still standing on it by then, same as a real trap you've
    /// already set off.
    pub fn check_fallingblocks_collision(&mut self, world: &mut World, room_index: usize, feet_block_pos: &BlockPos) -> bool {
        if self.fallingblock_patterns.is_empty() {
            return false;
        }

        // `feet_block_pos` is already resolved (by the caller, using an epsilon-adjusted floor
        // rather than a bare truncation) to the block the player's feet are actually resting on -
        // no "-1" here. A bare `position.y as i32` would be one cell too high for anything with a
        // sub-full-block top surface (a bottom-half slab's walkable surface sits at `y + 0.5`,
        // stairs/carpet similarly), since only an exact-integer feet height (a full block, or a
        // top-half slab) means "the support is one whole cell below".
        let player_feet_pos = *feet_block_pos;

        // Find the first pattern that has the block the player is standing on
        for (i, pattern) in self.fallingblock_patterns.iter().enumerate() {
            if self.pending_fallingblock_triggers.contains(&i) {
                continue; // already scheduled - waiting on the pulse, not re-armed by a re-step
            }

            let is_standing_on = pattern.blocks.iter().any(|block| {
                let pos = BlockPos { x: block.x, y: block.y, z: block.z };
                pos == player_feet_pos
            });

            if is_standing_on {
                // Inserted here and never removed, whether the schedule below has fired yet or
                // not - this doubles as both "don't re-arm while waiting to fall" and (once it
                // fires) "already dropped, never findable again", replacing what used to be an
                // actual `Vec::remove` at trigger time. That removal has to stay gone now that
                // more than one pattern's trigger can be in flight at once (each waiting on its
                // own scheduled tick): removing by index would shift every *other* still-pending
                // index that happened to be numerically larger, corrupting whichever pattern that
                // scheduled closure captured.
                self.pending_fallingblock_triggers.insert(i);

                // The visual/collision cue starts right now, on the step itself - not deferred
                // any further. Only the moment the floor actually becomes passable (letting the
                // player really fall through) waits the fixed delay below.
                self.begin_fallingblock_animation(world, i);

                const FALL_DELAY_TICKS: u32 = 5;
                world.server_mut().schedule(FALL_DELAY_TICKS, move |server| {
                    let Some(room) = server.dungeon.rooms.get_mut(room_index) else { return };
                    let world = &mut server.world;
                    room.finalize_fallingblock_drop(world, i);
                });
                return true;
            }
        }

        false
    }

    /// Runs the instant a player steps onto a falling block pattern - not deferred to the pulse.
    /// Swaps each block to `Barrier` (invisible, but still solid: the floor already looks like
    /// it's gone the moment you step on it, but you can't fall through it yet - that's
    /// `finalize_fallingblock_drop`'s job, once the shared cycle's pulse actually arrives) and
    /// spawns the cosmetic falling-block entity for each one, which runs its own independent
    /// 5-blocks-over-20-ticks descent regardless of when the real floor opens up (real Hypixel's
    /// same "the debris keeps falling after the hole opens" visual, not a gate on passability).
    fn begin_fallingblock_animation(&mut self, world: &mut World, pattern_index: usize) {
        if let Some(pattern) = self.fallingblock_patterns.get(pattern_index).cloned() {
            // Play sound effect
            for (_, player) in &mut world.players {
                let _ = player.write_packet(&crate::net::protocol::play::clientbound::SoundEffect {
                    sound: crate::server::utils::sounds::Sounds::RandomFizz.id(),
                    pos_x: pattern.blocks[0].x as f64 + 0.5,
                    pos_y: pattern.blocks[0].y as f64 + 0.5,
                    pos_z: pattern.blocks[0].z as f64 + 0.5,
                    volume: 5.0,
                    pitch: 0.49,
                });
            }

            // Spawn falling block entities for each block in the pattern
            for block in &pattern.blocks {
                let x = block.x;
                let y = block.y;
                let z = block.z;

                // Get the current block at this position
                let current_block = world.get_block_at(x, y, z);

                // Skip air blocks
                if matches!(current_block, crate::server::block::blocks::Blocks::Air) {
                    continue;
                }

                // Invisible but still solid - see this method's own doc comment for why this
                // isn't Air yet.
                world.set_block_at(crate::server::block::blocks::Blocks::Barrier, x, y, z);
                world.interactable_blocks.remove(&crate::server::block::block_position::BlockPos { x, y, z });

                // Spawn falling block entity for animation
                let _ = world.spawn_entity(
                    crate::server::utils::dvec3::DVec3::new(x as f64 + 0.5, y as f64 - crate::dungeon::room::fallingblocks::FALLING_FLOOR_ENTITY_OFFSET, z as f64 + 0.5),
                    {
                        let mut metadata = crate::server::entity::entity_metadata::EntityMetadata::new(crate::server::entity::entity_metadata::EntityVariant::Bat { hanging: false });
                        metadata.is_invisible = true;
                        metadata
                    },
                    crate::dungeon::room::fallingblocks::FallingFloorEntityImpl::new(current_block, 5.0, 20),
                );
            }
        }
    }

    /// Runs 5 ticks after the step that armed it (scheduled by `check_fallingblocks_collision`) -
    /// the moment the floor actually opens up and the player can fall through.
    /// `begin_fallingblock_animation` already handled the visual/sound cue and the
    /// solid-but-invisible `Barrier` swap back when the player first stepped on; this only needs
    /// to open the hole. Checks the block is still `Barrier` first (defensive - it should always
    /// be, since nothing else touches these positions in between) rather than blindly overwriting
    /// whatever's there. Not removed from `fallingblock_patterns` here either, for the same
    /// index-stability reason `pending_fallingblock_triggers` (already permanently marking this
    /// index) exists in the first place.
    fn finalize_fallingblock_drop(&mut self, world: &mut World, pattern_index: usize) {
        let Some(pattern) = self.fallingblock_patterns.get(pattern_index).cloned() else { return };
        for block in &pattern.blocks {
            let (x, y, z) = (block.x, block.y, block.z);
            if matches!(world.get_block_at(x, y, z), crate::server::block::blocks::Blocks::Barrier) {
                world.set_block_at(crate::server::block::blocks::Blocks::Air, x, y, z);
            }
        }
    }

    /// Process scheduled falling block removals (now handled by entities)
    pub fn process_scheduled_falling_removals(&mut self, _world: &mut World) {
        // This method is now empty since falling blocks are handled by entities
        // The scheduled_falling_removals field is kept for compatibility but not used
    }

    pub fn get_1x1_shape_and_type(segments: &[RoomSegment], dungeon_doors: &[Door]) -> (RoomShape, Direction) {
        let center_x = segments[0].x as i32 * 32 + 15 + DUNGEON_ORIGIN.0;
        let center_z = segments[0].z as i32 * 32 + 15 + DUNGEON_ORIGIN.1;

        // Actual doors found in the world
        let doors_opt = [
            (center_x, center_z - 16),
            (center_x + 16, center_z),
            (center_x, center_z + 16),
            (center_x - 16, center_z)
        ].iter().map(|pos| {
            dungeon_doors.iter()
                .find(|door| door.x == pos.0 && door.z == pos.1)
                .is_some()
        }).collect::<Vec<bool>>();

        let mut num: u8 = 0;
        for i in 0..4 {
            num <<= 1;
            num |= doors_opt[i] as u8;
        }

        // println!("{:04b} {:?}", num, doors_opt);

        match num {
            // Doors on all sides, never changes
            0b1111 => (RoomShape::OneByOneCross, Direction::North),
            // Dead end 1x1
            0b1000 => (RoomShape::OneByOneEnd, Direction::North),
            0b0100 => (RoomShape::OneByOneEnd, Direction::East),
            0b0010 => (RoomShape::OneByOneEnd, Direction::South),
            0b0001 => (RoomShape::OneByOneEnd, Direction::West),
            // Opposite doors
            0b0101 => (RoomShape::OneByOneStraight, Direction::North),
            0b1010 => (RoomShape::OneByOneStraight, Direction::East),
            // L bend
            0b0011 => (RoomShape::OneByOneBend, Direction::North),
            0b1001 => (RoomShape::OneByOneBend, Direction::East),
            0b1100 => (RoomShape::OneByOneBend, Direction::South),
            0b0110 => (RoomShape::OneByOneBend, Direction::West),
            // Triple door
            0b1011 => (RoomShape::OneByOneTriple, Direction::North),
            0b1101 => (RoomShape::OneByOneTriple, Direction::East),
            0b1110 => (RoomShape::OneByOneTriple, Direction::South),
            0b0111 => (RoomShape::OneByOneTriple, Direction::West),
            
            _ => (RoomShape::OneByOne, Direction::North),
        }
    }

    pub fn get_rotation_from_segments(segments: &[RoomSegment], dungeon_doors: &[Door]) -> Direction {
        let unique_x = segments.iter()
            .map(|segment| segment.x)
            .collect::<HashSet<usize>>();
        let unique_z = segments.iter()
            .map(|segment| segment.z)
            .collect::<HashSet<usize>>();

        let not_long = unique_x.len() > 1 && unique_z.len() > 1;

        match segments.len() {
            1 => {
                let (_, direction) = Room::get_1x1_shape_and_type(segments, dungeon_doors);
                direction
            },
            2 => match unique_z.len() == 1 {
                true => Direction::North,
                false => Direction::East,
            },
            3 => {  
                // L room
                if not_long {
                    let corner_value = segments.iter().find(|x| {
                        segments.iter().all(|y| {
                            x.x.abs_diff(y.x) + x.z.abs_diff(y.z) <= 1
                        })
                    }).expect("Invalid L room: Segments:");

                    let min_x = segments.iter().min_by(|a, b| a.x.cmp(&b.x)).unwrap().x;
                    let min_z = segments.iter().min_by(|a, b| a.z.cmp(&b.z)).unwrap().z;
                    let max_x = segments.iter().max_by(|a, b| a.x.cmp(&b.x)).unwrap().x;
                    let max_z = segments.iter().max_by(|a, b| a.z.cmp(&b.z)).unwrap().z;

                    if corner_value.x == min_x && corner_value.z == min_z {
                        return Direction::East
                    }
                    if corner_value.x == max_x && corner_value.z == min_z {
                        return Direction::South
                    }
                    if corner_value.x == max_x && corner_value.z == max_z {
                        return Direction::West
                    }
                    return Direction::North
                }

                match unique_z.len() == 1 {
                    true => Direction::North,
                    false => Direction::East,
                }
            },
            4 => {
                if unique_x.len() == 2 && unique_z.len() == 2 {
                    return Direction::North
                }

                match unique_z.len() == 1 {
                    true => Direction::North,
                    false => Direction::East,
                }
            },
            _ => unreachable!(),
        }
    }

    fn load_default(&self, world: &mut World) {
        for segment in self.segments.iter() {
            
            // Temporary for room colors, will be changed later on to paste saved room block states
            let block = match self.room_data.room_type {
                RoomType::Normal => Blocks::Stone { variant: 0 },
                RoomType::Blood => Blocks::Stone { variant: 0 },
                RoomType::Entrance => Blocks::Stone { variant: 0 },
                RoomType::Fairy => Blocks::Stone { variant: 0 },
                RoomType::Trap => Blocks::Stone { variant: 0 },
                RoomType::Yellow => Blocks::Stone { variant: 0 },
                RoomType::Puzzle => Blocks::Stone { variant: 0 },
                RoomType::Rare => Blocks::Stone { variant: 0 },
                RoomType::Boss => Blocks::Stone { variant: 0 },
            };

            world.fill_blocks(
                block,
                BlockPos {
                    x: segment.x as i32 * 32 + DUNGEON_ORIGIN.0,
                    y: self.room_data.bottom,
                    z: segment.z as i32 * 32 + DUNGEON_ORIGIN.1,
                },
                BlockPos {
                    x: segment.x as i32 * 32 + DUNGEON_ORIGIN.0 + 30,
                    y: self.room_data.bottom,
                    z: segment.z as i32 * 32 + DUNGEON_ORIGIN.1 + 30,
                }
            );

            // Merge to the side
            // if self.segments.contains(&(x+1, *z)) {
            //     world.fill_blocks(
            //         block,
            //         BlockPos {
            //             x: *x as i32 * 32 + 31 + DUNGEON_ORIGIN.0,
            //             y: self.room_data.bottom,
            //             z: *z as i32 * 32 + DUNGEON_ORIGIN.1,
            //         },
            //         BlockPos {
            //             x: *x as i32 * 32 + 31 + DUNGEON_ORIGIN.0,
            //             y: self.room_data.bottom,
            //             z: *z as i32 * 32 + DUNGEON_ORIGIN.1 + 30,
            //         }
            //     );
            // }
            // 
            // // // Merge below
            // if self.segments.contains(&(*x, z+1)) {
            //     world.fill_blocks(
            //         block,
            //         BlockPos {
            //             x: *x as i32 * 32 + DUNGEON_ORIGIN.0,
            //             y: self.room_data.bottom,
            //             z: *z as i32 * 32 + 31 + DUNGEON_ORIGIN.1,
            //         },
            //         BlockPos {
            //             x: * x as i32 * 32 + DUNGEON_ORIGIN.0 + 30,
            //             y: self.room_data.bottom,
            //             z: *z as i32 * 32 + 31 + DUNGEON_ORIGIN.1 + 30,
            //         }
            //     );
            // }
        }
    }

    /// Register levers for this room as interactable blocks
    pub fn register_levers(&self, world: &mut World) {
        for lever_data in &self.lever_data {
            let lever_pos = BlockPos {
                x: lever_data.lever[0],
                y: lever_data.lever[1],
                z: lever_data.lever[2],
            };
            
            // Register the lever as an interactable block
            world.interactable_blocks.insert(lever_pos, crate::server::block::block_interact_action::BlockInteractAction::Lever);
        }
    }

    pub fn load_into_world(&mut self, room_index: usize, world: &mut World) {
        if self.room_data.block_data.is_empty() {
            self.load_default(world);
            return;
        }

        let corner = self.get_corner_pos();
        
        // Load secrets from secrets.json
        self.json_secrets = crate::dungeon::room::secrets_loader::load_secrets_for_room(
            &self.room_data.name,
            corner,
            self.rotation
        );
        
        // Register levers for this room
        self.register_levers(world);

        // Three Weirdos' NPCs are deliberately NOT spawned here - see `Room::weirdos_spawned`'s
        // doc comment. `three_weirdos::setup` is now called from `Dungeon::tick`'s room-entry
        // hook instead, the first time a player actually crosses into the room.

        // Spawns the Creeper Beams floating creeper + registers its 22 lanterns, if this is
        // that room - no-op for every other room.
        crate::dungeon::room::creeper_beams::setup(self, room_index, world);

        // Generates the Teleport Maze puzzle's pad-link graph, if this is that room - no-op for
        // every other room.
        crate::dungeon::room::teleport_maze::setup(self, room_index, world);

        // Picks this dungeon's Ice Fill obstacle patterns and spawns its first floor, if this is
        // that room - no-op for every other room.
        crate::dungeon::room::ice_fill::setup(self, room_index, world);

        // Picks Water Board's own real maze pattern (purely internal variety - this stays exactly
        // ONE room_data_storage entry, see waterboard.rs's own module doc comment for why) and
        // swaps `self.room_data.block_data` to match, before the generic block-placement loop
        // below reads it - no-op for every other room. Guarded by `id` NOT already carrying a
        // `water_board_pattern_N` tag: `practice::find_room_data`'s own `water_board_N` special
        // case already calls `waterboard::apply_pattern` deterministically on the `RoomData` it
        // hands to `Room::new`, before this function ever runs on it - without this guard, this
        // call would silently REROLL and overwrite that deliberate choice with a fresh random one
        // every time, since it only ever checked `name`, which `apply_pattern` never changes.
        if self.room_data.name == "Water Board" && !self.room_data.id.starts_with("water_board_pattern_") {
            crate::dungeon::room::waterboard::pick_pattern(&mut self.room_data);
        }

        // Special placement for Gold room
        if self.room_data.name == "Gold" {
            let pos = BlockPos { x: 15, y: 100, z: 15 }.rotate(self.rotation);
            world.set_block_at(Blocks::GoldBlock, corner.x + pos.x, 100, corner.z + pos.z);
        }
        
        // Special placement for Redstone Key room
        if self.room_data.name == "Redstone Key" {
            // Determine spawn position based on room orientation
            // If room is facing north/south (has door on north/south), spawn at 19/66/7
            // If room is facing east/west, spawn at 10/70/26
            let (rel_x, rel_y, rel_z) = match self.rotation {
                Direction::North | Direction::South => (19, 66, 7),
                Direction::East | Direction::West => (10, 70, 26),
                Direction::Up | Direction::Down => (19, 66, 7), // Fallback (shouldn't happen)
            };
            
            let skull_pos = BlockPos { x: rel_x, y: rel_y, z: rel_z }.rotate(self.rotation);
            let world_pos = BlockPos {
                x: corner.x + skull_pos.x,
                y: skull_pos.y,
                z: corner.z + skull_pos.z,
            };
            
            // Spawn the skull block
            world.set_block_at(
                Blocks::Skull { direction: Direction::Up, no_drop: false },
                world_pos.x,
                world_pos.y,
                world_pos.z
            );
            
            // Send UpdateBlockEntity with skull NBT
            use crate::net::protocol::play::clientbound::UpdateBlockEntity;
            use crate::server::utils::nbt::serialize::serialize_nbt;
            use crate::dungeon::room::secrets::DungeonSecret;
            let skull_owner = DungeonSecret::create_redstone_key_skull_nbt();
            let full_te_nbt = crate::server::utils::nbt::nbt::NBT::with_nodes(vec![
                crate::server::utils::nbt::nbt::NBT::string("id", "Skull"),
                crate::server::utils::nbt::nbt::NBT::int("x", world_pos.x),
                crate::server::utils::nbt::nbt::NBT::int("y", world_pos.y),
                crate::server::utils::nbt::nbt::NBT::int("z", world_pos.z),
                crate::server::utils::nbt::nbt::NBT::byte("SkullType", 3), // 3 = player head
                skull_owner,
            ]);
            let nbt_bytes = serialize_nbt(&full_te_nbt);
            let update_packet = UpdateBlockEntity {
                block_pos: world_pos,
                action: 4, // 4 = skull update in 1.8
                nbt_data: Some(nbt_bytes.clone()),
            };
            for (_, player) in &mut world.players {
                player.write_packet(&update_packet);
            }
            let chunk_x = world_pos.x >> 4;
            let chunk_z = world_pos.z >> 4;
            if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                chunk.packet_buffer.write_packet(&update_packet);
            }
            
            // Register as interactable block
            world.interactable_blocks.insert(world_pos, crate::server::block::block_interact_action::BlockInteractAction::RedstoneKeySkull {
                room_index: 0, // Not used in interaction handler
            });
        }
        
        // Special placement for Golden Oasis room
        if self.room_data.name == "Golden Oasis" {
            // Skull always spawns at 12 71 8 (no rotation, always same place)
            let skull_rel_pos = BlockPos { x: 12, y: 71, z: 8 };
            let skull_world_pos = BlockPos {
                x: corner.x + skull_rel_pos.x,
                y: skull_rel_pos.y,
                z: corner.z + skull_rel_pos.z,
            };
            
            // Spawn the skull block
            world.set_block_at(
                Blocks::Skull { direction: Direction::Up, no_drop: false },
                skull_world_pos.x,
                skull_world_pos.y,
                skull_world_pos.z
            );
            
            // Send UpdateBlockEntity with skull NBT
            use crate::net::protocol::play::clientbound::UpdateBlockEntity;
            use crate::server::utils::nbt::serialize::serialize_nbt;
            use crate::dungeon::room::secrets::DungeonSecret;
            let skull_owner = DungeonSecret::create_redstone_key_skull_nbt();
            let full_te_nbt = crate::server::utils::nbt::nbt::NBT::with_nodes(vec![
                crate::server::utils::nbt::nbt::NBT::string("id", "Skull"),
                crate::server::utils::nbt::nbt::NBT::int("x", skull_world_pos.x),
                crate::server::utils::nbt::nbt::NBT::int("y", skull_world_pos.y),
                crate::server::utils::nbt::nbt::NBT::int("z", skull_world_pos.z),
                crate::server::utils::nbt::nbt::NBT::byte("SkullType", 3), // 3 = player head
                skull_owner,
            ]);
            let nbt_bytes = serialize_nbt(&full_te_nbt);
            let update_packet = UpdateBlockEntity {
                block_pos: skull_world_pos,
                action: 4, // 4 = skull update in 1.8
                nbt_data: Some(nbt_bytes.clone()),
            };
            for (_, player) in &mut world.players {
                player.write_packet(&update_packet);
            }
            let chunk_x = skull_world_pos.x >> 4;
            let chunk_z = skull_world_pos.z >> 4;
            if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                chunk.packet_buffer.write_packet(&update_packet);
            }
            
            // Register as interactable block
            world.interactable_blocks.insert(skull_world_pos, crate::server::block::block_interact_action::BlockInteractAction::RedstoneKeySkull {
                room_index: 0, // Not used in interaction handler
            });
        }
        
        // Special placement for Redstone Crypt room
        if self.room_data.name == "Redstone Crypt" {
            // Skull always spawns at 4 71 4 (rotated based on room rotation)
            let skull_rel_pos = BlockPos { x: 4, y: 71, z: 4 }.rotate(self.rotation);
            let skull_world_pos = BlockPos {
                x: corner.x + skull_rel_pos.x,
                y: skull_rel_pos.y,
                z: corner.z + skull_rel_pos.z,
            };
            
            // Spawn the skull block
            world.set_block_at(
                Blocks::Skull { direction: Direction::Up, no_drop: false },
                skull_world_pos.x,
                skull_world_pos.y,
                skull_world_pos.z
            );
            
            // Send UpdateBlockEntity with skull NBT
            use crate::net::protocol::play::clientbound::UpdateBlockEntity;
            use crate::server::utils::nbt::serialize::serialize_nbt;
            use crate::dungeon::room::secrets::DungeonSecret;
            let skull_owner = DungeonSecret::create_redstone_key_skull_nbt();
            let full_te_nbt = crate::server::utils::nbt::nbt::NBT::with_nodes(vec![
                crate::server::utils::nbt::nbt::NBT::string("id", "Skull"),
                crate::server::utils::nbt::nbt::NBT::int("x", skull_world_pos.x),
                crate::server::utils::nbt::nbt::NBT::int("y", skull_world_pos.y),
                crate::server::utils::nbt::nbt::NBT::int("z", skull_world_pos.z),
                crate::server::utils::nbt::nbt::NBT::byte("SkullType", 3), // 3 = player head
                skull_owner,
            ]);
            let nbt_bytes = serialize_nbt(&full_te_nbt);
            let update_packet = UpdateBlockEntity {
                block_pos: skull_world_pos,
                action: 4, // 4 = skull update in 1.8
                nbt_data: Some(nbt_bytes.clone()),
            };
            for (_, player) in &mut world.players {
                player.write_packet(&update_packet);
            }
            let chunk_x = skull_world_pos.x >> 4;
            let chunk_z = skull_world_pos.z >> 4;
            if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                chunk.packet_buffer.write_packet(&update_packet);
            }
            
            // Register as interactable block
            world.interactable_blocks.insert(skull_world_pos, crate::server::block::block_interact_action::BlockInteractAction::RedstoneKeySkull {
                room_index: 0, // Not used in interaction handler
            });
        }
        

        for (i, block) in self.room_data.block_data.iter().enumerate() {
            if *block == Blocks::Air {
                continue;
            }
            // not sure if editing room data might ruin something,
            // so to be safe im just cloning it
            let mut block = block.clone();
            block.rotate(self.rotation);

            let ind = i as i32;

            let x = ind % self.room_data.width;
            let z = (ind / self.room_data.width) % self.room_data.length;
            let y = self.room_data.bottom + ind / (self.room_data.width * self.room_data.length);

            let bp = BlockPos { x, y, z }.rotate(self.rotation);

            world.set_block_at(block, corner.x + bp.x, y, corner.z + bp.z);
        }

        // TEMPORARY test-only overlay - see boulder_test.rs's own doc comment. Not the real
        // puzzle. Deliberately runs after the block-placement loop just above, same reasoning as
        // Tic Tac Toe's own post-placement hook right below: this needs to *override* the room's
        // Picks one of the 8 real captured box layouts, places its boxes, and registers the
        // real reward chest, if this is that room - no-op for every other room. Deliberately runs
        // after the block-placement loop just above: it needs to override the room's own normal
        // static blocks wherever a box sits.
        crate::dungeon::room::boulder::setup(self, room_index, world);

        // Spawns the Higher or Lower puzzle's 10 blazes with randomized (but position-independent)
        // HP, if this is that room - no-op for every other room.
        crate::dungeon::room::blaze::setup(self, room_index, world);


        // Spawns the Tic Tac Toe board's 9 Item Frames (display only for now - see
        // `tic_tac_toe_state`'s doc comment), if this is that room - no-op for every other room.
        // Deliberately runs after the block-placement loop just above (not up with the other
        // puzzle setups) - it needs to read back each button's actual placed (rotated) direction
        // so the frames it later spawns can match, rather than recomputing that rotation itself
        // and risking disagreeing with whatever `Blocks::rotate` actually produced for the
        // buttons - see `tic_tac_toe::setup`'s doc comment.
        crate::dungeon::room::tic_tac_toe::setup(self, room_index, world);

        // Registers every mushroom secret's bottom/top blocks as interactable ONCE, here at room
        // load time, instead of every tick for every player the game currently considers "in"
        // this room (the old approach, in `Dungeon`'s per-tick player loop) - that depended on
        // continuous 2D-grid-cell room tracking by player position, which stops the instant a
        // player is teleported up to a set's own hidden loft if that loft's X/Z happens to fall
        // outside this room's own grid footprint (common for a hidden secret spot). Per explicit
        // bug report: a mushroom became permanently non-interactable after exactly one round
        // trip - registering once, unconditionally, for the room's whole lifetime removes that
        // entire dependency on live player tracking.
        for (idx, set) in self.mushroom_sets.iter().enumerate() {
            for &bp in &set.bottom {
                world.interactable_blocks.insert(bp, crate::server::block::block_interact_action::BlockInteractAction::MushroomBottom { set_index: idx });
            }
            for &bp in &set.top {
                world.interactable_blocks.insert(bp, crate::server::block::block_interact_action::BlockInteractAction::MushroomTop);
            }
        }
    }

    // pub fn get_world_pos(&self, position: DVec3) -> DVec3 {
    //     let corner = self.get_corner_pos();
    //     position.clone()
    //         .rotate(self.rotation)
    //         .add_x(corner.x as f64)
    //         .add_z(corner.z as f64)
    // }

    pub fn get_world_block_pos(&self, room_pos: &BlockPos) -> BlockPos {
        let corner = self.get_corner_pos();

        room_pos.clone()
            .rotate(self.rotation)
            .add_x(corner.x)
            .add_z(corner.z)
    }

    /// Inverse of `get_world_block_pos` - converts a world-space position back into this room's
    /// own pre-rotation local coordinates. Useful for "has the player reached this specific
    /// captured spot" checks (e.g. Quiz's entry-threshold gate) where the interesting comparison
    /// is against a fixed local coordinate, not a world one that would need re-deriving per
    /// rotation.
    ///
    /// `BlockPos::rotate`'s 4 cases are their own inverses except East/West, which undo each
    /// other (rotating by East then West, in either order, is the identity) - North and South
    /// each rotate 0°/180°, so applying either twice returns the original point, while East/West
    /// are 90°/270° and only cancel against one another. So the inverse of "this room was placed
    /// rotated by `self.rotation`" is just "rotate by that same value, except swap East<->West".
    pub fn get_local_block_pos(&self, world_pos: &BlockPos) -> BlockPos {
        let corner = self.get_corner_pos();
        let offset = BlockPos {
            x: world_pos.x - corner.x,
            y: world_pos.y,
            z: world_pos.z - corner.z,
        };
        let inverse_rotation = match self.rotation {
            Direction::East => Direction::West,
            Direction::West => Direction::East,
            other => other,
        };
        offset.rotate(inverse_rotation)
    }

    /// World-space AABB (inclusive) this room's blocks occupy - built from the same
    /// `get_world_block_pos` transform actually used to place blocks, rather than re-deriving
    /// the rotation/axis-swap logic by hand, so it can't disagree with it for East/West-rotated
    /// rooms. Used by `ai::pathfinding` to bound the A* search volume to just this room.
    pub fn get_world_bounds(&self) -> (BlockPos, BlockPos) {
        let rd = &self.room_data;
        let corners = [
            BlockPos::new(0, 0, 0),
            BlockPos::new(rd.width - 1, 0, 0),
            BlockPos::new(0, 0, rd.length - 1),
            BlockPos::new(rd.width - 1, 0, rd.length - 1),
        ];

        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_z = i32::MAX;
        let mut max_z = i32::MIN;
        for corner in corners {
            let world_pos = self.get_world_block_pos(&corner);
            min_x = min_x.min(world_pos.x);
            max_x = max_x.max(world_pos.x);
            min_z = min_z.min(world_pos.z);
            max_z = max_z.max(world_pos.z);
        }

        (
            BlockPos::new(min_x, rd.bottom, min_z),
            BlockPos::new(max_x, rd.bottom + rd.height - 1, max_z),
        )
    }

    /// Spawn locked chests for this room
    pub fn spawn_locked_chests(
        &self,
        world: &mut World,
        locked_chests: &mut std::collections::HashMap<BlockPos, LockedChestState>,
        lever_to_chests: &mut std::collections::HashMap<BlockPos, Vec<BlockPos>>,
    ) {
        if let Some(locked_chest_entries) = get_room_locked_chests(&self.room_data.name) {
            let corner = self.get_corner_pos();
            let mut rng = seeded_rng();

            for entry in locked_chest_entries {
                // Convert relative chest position to world coordinates
                let relative_chest_pos = BlockPos {
                    x: entry.x,
                    y: entry.y,
                    z: entry.z,
                };
                let rotated_chest = relative_chest_pos.rotate(self.rotation);
                let chest_world_pos = BlockPos {
                    x: corner.x + rotated_chest.x,
                    y: rotated_chest.y, // Y coordinates are absolute
                    z: corner.z + rotated_chest.z,
                };

                // Convert relative lever position to world coordinates
                let relative_lever_pos = BlockPos {
                    x: entry.lever[0],
                    y: entry.lever[1],
                    z: entry.lever[2],
                };
                let rotated_lever = relative_lever_pos.rotate(self.rotation);
                let lever_world_pos = BlockPos {
                    x: corner.x + rotated_lever.x,
                    y: rotated_lever.y, // Y coordinates are absolute
                    z: corner.z + rotated_lever.z,
                };

                // 50/50 chance to be locked
                let locked = rng.random::<f64>() < 0.5;

                // Place the chest block
                let facing_direction = facing_string_to_direction(&entry.facing);
                let rotated_facing = facing_direction.rotate(self.rotation);
                world.set_block_at(
                    Blocks::Chest { direction: rotated_facing },
                    chest_world_pos.x,
                    chest_world_pos.y,
                    chest_world_pos.z,
                );

                // Register chest as interactable
                world.interactable_blocks.insert(
                    chest_world_pos,
                    crate::server::block::block_interact_action::BlockInteractAction::Chest {
                        secret: std::rc::Rc::new(std::cell::RefCell::new(
                            crate::dungeon::room::secrets::DungeonSecret::new(
                                crate::dungeon::room::secrets::SecretType::Chest {
                                    direction: rotated_facing,
                                },
                                chest_world_pos,
                                0.0,
                            ),
                        )),
                    },
                );

                // Store locked chest state
                locked_chests.insert(
                    chest_world_pos,
                    LockedChestState {
                        locked,
                        lever_world_pos,
                    },
                );

                // Add to lever -> chests mapping
                lever_to_chests
                    .entry(lever_world_pos)
                    .or_insert_with(Vec::new)
                    .push(chest_world_pos);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `get_local_block_pos` is untested directly here (it needs a real `Room`, which needs a
    /// full `RoomData`/segment setup this file has no lightweight constructor for) - but its
    /// entire correctness rests on one claim from its own doc comment: rotating by `self.rotation`
    /// then by that same value with East/West swapped returns the original point, for all 4
    /// rotations. This tests that claim directly against `BlockPos::rotate` itself.
    #[test]
    fn rotation_inverse_round_trips_for_every_direction() {
        fn inverse(rotation: Direction) -> Direction {
            match rotation {
                Direction::East => Direction::West,
                Direction::West => Direction::East,
                other => other,
            }
        }

        let point = BlockPos { x: 7, y: 3, z: -2 };
        for rotation in [Direction::North, Direction::East, Direction::South, Direction::West] {
            let rotated = point.rotate(rotation);
            let back = rotated.rotate(inverse(rotation));
            assert_eq!(back, point, "rotation {rotation:?} did not invert cleanly (rotated to {rotated:?})");
        }
    }
}


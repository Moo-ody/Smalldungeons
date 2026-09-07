//! Ice Fill puzzle, ported from RustClear's own `IceFillPuzzle` (that file's own comment: "this
//! is fitting. bad code for a bad puzzle") - the core mechanic is carried over as-is: walk every
//! remaining ice tile of the current floor exactly once (no diagonals, no repeats, no skipping),
//! across 3 sequential floors.
//!
//! Obstacle patterns are the user's own live in-game captures (`/ifs`, the ChatTriggers
//! `RoomsWorldHelper` module's capture command - see `src/room_data/misc/puzzles/
//! icefillpatterns.json`), not derived/guessed - confirmed real polished andesite blocks, one
//! tile above the ice tile each blocks (matching `Blocks::Stone { variant: 6 }` used below).
//! Real counts are 4/6/5 for floors 1/2/3 (independently cross-checked against Odin's own real,
//! actively-maintained solver - `IceFillSolver.kt`/`iceFillFloors.json`) - floor 1 currently only
//! has 2 of its 4 captured, wired in as-is per explicit request rather than waiting for full
//! coverage; it'll just have fewer random options than floors 2/3 until the rest are captured.
//!
//! `FLOOR_*_FOOTPRINT` below is the real, corrected footprint data (rebuilt from that same Odin
//! data after cross-checking showed RustClear's own arrays were incomplete: a naive rectangular
//! assumption for floors 2 and 3 missed real tiles, e.g. floor 2's actual footprint has a partial
//! extra row at z=11 that isn't part of a clean rectangle).
//!
//! Per explicit request, this code never places the floor's base Ice - this project's own captured
//! room (`47,ice_fill,-132,-492.json`) already has real Ice at these positions as part of the
//! room's normal static layout, the same as any other block in the room, so there's nothing to
//! (re-)place there. The only blocks this module ever writes are the obstacle blocks (which vary
//! per randomly-chosen pattern, so genuinely need active placement/management - see
//! `IceFillState::obstacle_choice`) and, one tile at a time as gameplay actually changes it, the
//! Ice<->PackedIce progress marker on a tile a player has/hasn't walked yet (`reset_progress`
//! undoes exactly the tiles a failed attempt actually touched, never the floor as a whole).
//!
//! Continuous per-tick position tracking (not click-based) - same reasoning and same `Room::tick`
//! hook `teleport_maze.rs` already established for this exact style of puzzle (a tile triggers by
//! being *walked onto*, not right-clicked, so there's no single interactable block position to
//! hang state off of). Set up at room *load*, not gated behind room-entry - unlike Quiz/Three
//! Weirdos (which defer specifically because of dialogue/NPC-spawn timing), this is just static
//! block placement with nothing to spoil by being visible early, same as Teleport Maze's own pads.
//!
//! Not yet ported: RustClear's own completion visual (iron bars sliding down as an animated gate,
//! via its ECS `MovingBlockBehaviour`) is simplified to a plain block-clear, matching how every
//! other puzzle here (Ice Path, Teleport Maze) opens its own gate/wall - no equivalent animated-
//! block-riding-an-invisible-mount helper exists in this codebase to reuse instead. Whether a real
//! reward chest exists beyond the gate isn't handled either - RustClear's own `complete()` doesn't
//! spawn one, only opens the gate, so there was nothing to port on that front; if one needs adding
//! it'll need its own research pass the way Ice Path's/Teleport Maze's reward chests got.

use crate::dungeon::room::room::Room;
use crate::dungeon::room::secrets::{DungeonSecret, SecretType};
use crate::net::protocol::play::clientbound::SoundEffect;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::player::player::ClientId;
use crate::server::utils::direction::Direction;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::{Cell, RefCell};

/// How long (ticks) the floor stays broken (see `break_floor`) before restoring, per explicit
/// request - RustClear's own donor value was 60 (`in_ticks: 60`, 3s @ 20 TPS); this is 40 (2s).
const FAIL_BREAK_TICKS: u64 = 40;

// --- Real footprint data, rebuilt from Odin's `iceFillFloors.json` - see the module doc comment
// for how and why this replaces RustClear's own (incomplete) arrays. Each tuple is a room-
// relative (x, z); the y for a whole floor is fixed (`FLOOR_Y`).

const FLOOR_Y: [i32; 3] = [69, 70, 71];

const FLOOR_0_FOOTPRINT: &[(i32, i32)] = &[(14,7), (14,8), (14,9), (15,7), (15,8), (15,9), (15,10), (16,7), (16,8), (16,9)];
// (13,11)/(15,11)/(17,11) deliberately excluded here, and (15,18)/(17,18)/(17,26)/(19,18)/(19,26)
// from floor 3 below - confirmed via a direct block-by-block decode of this project's own captured
// room to be permanent entrance-threshold architecture (real stair/andesite blocks, not ice), never
// referenced by any of the user's real `/ifs` obstacle captures either. Including them previously
// caused two live bugs: `activate_floor(..., true)` stamped `Blocks::Ice` over that real
// architecture on every fail-restore, and normal foot traffic across the entrance falsely triggered
// "wrong block" fails since these tiles aren't actually part of the walk path.
//
// (13,13)/(13,14)/(13,15) below (floor 2) and (16,25)/(18,23)/(18,24) in floor 3 were the opposite
// mistake - real ice tiles the same decode found actually present in the room that this array was
// missing. An omitted tile isn't just "walkable but unscored": since `tick_player` early-returns
// for any position outside `footprint` without updating `tracked`, crossing one of these gaps left
// `tracked` pointing at the tile from *before* the gap, so the next real tile onto the walk path
// registered as more than 1 tile away and falsely tripped the diagonal/wrong-block fail.
const FLOOR_1_FOOTPRINT: &[(i32, i32)] = &[(13,12), (13,13), (13,14), (13,15), (13,16), (14,12), (14,13), (14,14), (14,15), (14,16), (15,12), (15,13), (15,14), (15,15), (15,16), (15,17), (16,12), (16,13), (16,14), (16,15), (16,16), (17,12), (17,13), (17,14), (17,15), (17,16)];
const FLOOR_2_FOOTPRINT: &[(i32, i32)] = &[(12,19), (12,20), (12,21), (12,22), (12,23), (12,24), (12,25), (13,19), (13,20), (13,21), (13,22), (13,23), (13,24), (13,25), (14,19), (14,20), (14,21), (14,22), (14,23), (14,24), (14,25), (15,19), (15,20), (15,21), (15,22), (15,23), (15,24), (15,25), (15,26), (16,19), (16,20), (16,21), (16,22), (16,23), (16,24), (16,25), (17,19), (17,20), (17,21), (17,22), (17,23), (17,24), (17,25), (18,19), (18,20), (18,21), (18,22), (18,23), (18,24), (18,25)];

const FOOTPRINTS: [&[(i32, i32)]; 3] = [FLOOR_0_FOOTPRINT, FLOOR_1_FOOTPRINT, FLOOR_2_FOOTPRINT];

// --- Real obstacle patterns, captured live in-game via `/ifs` (this project's own
// `RoomsWorldHelper` ChatTriggers module) - see `src/room_data/misc/puzzles/icefillpatterns.json`
// for the raw capture. Each tuple is the (x, z) of an ice tile that pattern blocks - the actual
// obstacle block is placed one above it (`y + 1`), matching RustClear's own
// `obstacles.contains(&(block + IVec3::Y))` convention. Floor 1 only has 2 of its real 4 patterns
// captured so far - see the module doc comment.

const FLOOR_0_OBSTACLE_PATTERNS: &[&[(i32, i32)]] = &[
    &[(16,7), (14,7), (14,8), (14,9)],
    &[(14,7), (16,9)],
];

const FLOOR_1_OBSTACLE_PATTERNS: &[&[(i32, i32)]] = &[
    &[(15,13), (14,15), (16,16), (17,16), (17,15), (17,12)],
    &[(15,13), (16,13), (14,15), (15,15)],
    &[(15,13), (14,15)],
    &[(14,14), (16,13), (16,16), (17,16)],
    &[(14,14), (15,14)],
    &[(15,14), (16,14)],
];

const FLOOR_2_OBSTACLE_PATTERNS: &[&[(i32, i32)]] = &[
    &[(16,25), (15,23), (14,23), (18,19)],
    &[(13,20), (15,20), (15,21), (14,22), (15,22), (15,23), (16,22), (17,22), (16,25), (12,25)],
    &[(16,25), (14,24), (13,24), (17,22), (16,22), (16,21), (14,20), (14,19)],
    &[(16,25), (15,23), (15,20), (14,21), (13,21)],
    &[(14,21), (15,21), (16,21), (16,24), (16,25), (14,25)],
];

const OBSTACLE_PATTERNS: [&[&[(i32, i32)]]; 3] = [FLOOR_0_OBSTACLE_PATTERNS, FLOOR_1_OBSTACLE_PATTERNS, FLOOR_2_OBSTACLE_PATTERNS];

/// Room-relative gate blocks that clear on full completion - RustClear's own `complete()` range
/// (`x: 13..=17, y: 74..=79, z: 27`), simplified to a plain clear (see the module doc comment for
/// why the animated-iron-bars version wasn't ported).
const GATE_MIN: BlockPos = BlockPos { x: 13, y: 74, z: 27 };
const GATE_MAX: BlockPos = BlockPos { x: 17, y: 79, z: 27 };

/// The two north-facing "blessing" chests behind the gate - per explicit request, these already
/// exist as part of the room's own static block data (this module never places them), start
/// non-interactable, and only become openable once all 3 floors are cleared. Opening *both* -
/// not either one alone - is the puzzle's actual win condition (see `open_reward_chests` and
/// `DungeonSecret::puzzle_chest_group`), matching how Teleport Maze's single reward chest already
/// works for the one-chest case.
const CHEST_POSITIONS: [BlockPos; 2] = [
    BlockPos { x: 16, y: 75, z: 29 },
    BlockPos { x: 14, y: 75, z: 29 },
];

/// The real captured blessing skull texture - same constant (by value) as `tic_tac_toe.rs`'s
/// `REWARD_BLESSING_TEXTURE`, `ice_path.rs`'s `REWARD_BLESSING_TEXTURE`, and `teleport_maze.rs`'s
/// `SOLVE_BLESSING_TEXTURE`. Each file keeps its own copy rather than importing a shared one - see
/// `DungeonSecret::blessing_texture`'s doc comment for what setting this actually does. Not
/// `secrets::WITHER_ESSENCE_TEXTURE` - that's a different, wither-essence-specific skin, mistakenly
/// used here originally.
const REWARD_BLESSING_TEXTURE: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZTkzZTIwNjg2MTc4NzJjNTQyZWNkYTFkMjdkZjRlY2U5MWM2OTk5MDdiZjMyN2M0ZGRiODUzMDk0MTJkMzkzOSJ9fX0=";

#[derive(Debug)]
pub struct IceFillState {
    room_index: usize,
    /// Which obstacle pattern was randomly picked for each floor - fixed once per dungeon, not
    /// re-rolled on a failed retry (matching the real server, per the module doc comment).
    obstacle_choice: [usize; 3],
    current_floor: usize,
    layer: LayerState,
}

#[derive(Debug)]
enum LayerState {
    /// A failed attempt breaks the floor (`break_floor`) and waits `FAIL_BREAK_TICKS` before
    /// restoring it (same obstacle pattern, Ice put back) - `resume_at_tick` is `world.tick_count`
    /// at that point, checked each `tick()` rather than hand-rolling a countdown.
    Respawning { resume_at_tick: u64 },
    /// All 3 floors cleared - per explicit request, the puzzle is now frozen for good: `tick()`
    /// stops calling `tick_player` entirely, so nothing can ever fail/break a floor again (the
    /// bug this fixes: without this state, `layer` was left as `Active` with an already-empty
    /// `remaining` after floor 3, so any further step anyone took registered as "wrong block" and
    /// re-triggered `break_floor` on a puzzle that had already been solved).
    Done,
    Active {
        /// World positions of every tile in the active floor's full footprint (obstacle-covered
        /// or not) - used only to tell "is this player even on the puzzle at all" from "they're
        /// just walking past it".
        footprint: std::collections::HashSet<BlockPos>,
        /// World positions of tiles not yet correctly stepped on this attempt - shrinks as
        /// players walk it correctly; success is this becoming empty.
        remaining: std::collections::HashSet<BlockPos>,
        /// Last floor-tile position each tracked player was standing on this attempt - `None`
        /// entries aren't kept; a client simply absent from the map means "not tracked yet",
        /// which is what skips the adjacency check on a player's first step onto a fresh floor.
        tracked: HashMap<ClientId, BlockPos>,
    },
}

/// Sets up the Ice Fill puzzle for `room` if it actually is one - spawns floor 1 immediately.
/// No-op for every other room. Called once from `Room::load_into_world`, same timing as
/// `teleport_maze::setup` - see the module doc comment for why this doesn't need to wait for room
/// entry the way Quiz/Three Weirdos do.
pub fn setup(room: &mut Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Ice Fill" {
        return;
    }

    let mut rng = rand::rng();
    let obstacle_choice = std::array::from_fn(|floor| rand::Rng::random_range(&mut rng, 0..OBSTACLE_PATTERNS[floor].len()));

    // Per explicit request: every floor's obstacle pattern is placed the instant the room loads,
    // not progressively revealed floor-by-floor as each one is cleared. The obstacle choice for
    // the whole dungeon is already fixed right above (never re-rolled per floor/attempt), and the
    // room's own captured static data already has real Ice everywhere for all 3 floors regardless
    // of progress - so there was never a real reason to withhold floors 2/3's obstacle blocks; a
    // player (or a solver like Odin) should be able to see every floor's actual layout up front,
    // matching the real Hypixel mechanic.
    for floor in 0..3 {
        place_obstacles(room, world, floor, obstacle_choice[floor]);
    }

    let state = Rc::new(RefCell::new(IceFillState {
        room_index,
        obstacle_choice,
        current_floor: 0,
        layer: LayerState::Respawning { resume_at_tick: 0 }, // spawn_layer runs immediately below
    }));

    room.ice_fill_state = Some(state.clone());
    activate_floor(&state, room, world, false);
}

/// Places the current floor's obstacle blocks and resets its `Active` tracking state.
/// `restore_ice` controls whether the floor's Ice tiles are (re-)placed too: `false` for a fresh
/// floor's first activation (`setup`/`on_floor_cleared`'s advance-to-next-floor) - the room's own
/// captured static data already has real Ice there, so there's nothing to place - `true` only for
/// `on_floor_failed`'s post-break restore, since that's the one case this module *does*
/// deliberately blank the floor's Ice to air first (see its own doc comment) and so is the one
/// case that actually needs to put it back.
fn activate_floor(state_rc: &Rc<RefCell<IceFillState>>, room: &Room, world: &mut World, restore_ice: bool) {
    let (floor, pattern_index) = {
        let state = state_rc.borrow();
        (state.current_floor, state.obstacle_choice[state.current_floor])
    };

    let y = FLOOR_Y[floor];
    let obstacles: &[(i32, i32)] = OBSTACLE_PATTERNS[floor][pattern_index];

    let mut footprint = std::collections::HashSet::new();
    let mut remaining = std::collections::HashSet::new();
    for &(x, z) in FOOTPRINTS[floor] {
        let world_pos = room.get_world_block_pos(&BlockPos::new(x, y, z));
        if restore_ice {
            world.set_block_at(Blocks::Ice, world_pos.x, world_pos.y, world_pos.z);
        }
        footprint.insert(world_pos);
        if !obstacles.contains(&(x, z)) {
            remaining.insert(world_pos);
        }
    }
    // Idempotent - on a fresh floor's first activation these are already sitting there from
    // `setup`'s up-front placement (see its own doc comment); this only actually does anything on
    // the post-fail-break restore path, where `break_floor` blanked them to air.
    place_obstacles(room, world, floor, pattern_index);

    state_rc.borrow_mut().layer = LayerState::Active { footprint, remaining, tracked: HashMap::new() };
}

/// Places one floor's obstacle blocks for its chosen pattern - one block above (`y + 1`) the ice
/// tile each blocks, confirmed by the user's own live `/ifs` capture. Called up front for all 3
/// floors from `setup` (see its own doc comment for why), and again from `activate_floor` to
/// restore them after a fail-break blanks them to air.
fn place_obstacles(room: &Room, world: &mut World, floor: usize, pattern_index: usize) {
    let y = FLOOR_Y[floor];
    for &(x, z) in OBSTACLE_PATTERNS[floor][pattern_index] {
        let world_pos = room.get_world_block_pos(&BlockPos::new(x, y + 1, z));
        world.set_block_at(Blocks::Stone { variant: 6 }, world_pos.x, world_pos.y, world_pos.z); // polished andesite
    }
}

/// A wrong step or diagonal move - per explicit request, breaks *every* Ice tile and obstacle
/// block on the current floor to air (not just the ones a player actually walked), a clear "you
/// failed" tell, then `activate_floor(..., true)` puts real Ice and the same obstacle pattern back
/// once `FAIL_BREAK_TICKS` elapses (checked in `tick()`). Per explicit request, every tile that's
/// actually still ice-family (`Ice` or a walked `PackedIce`) plays a glass-break sound as it goes.
fn break_floor(room: &Room, world: &mut World, floor: usize, pattern_index: usize) {
    let y = FLOOR_Y[floor];
    for &(x, z) in FOOTPRINTS[floor] {
        let world_pos = room.get_world_block_pos(&BlockPos::new(x, y, z));
        if matches!(world.get_block_at(world_pos.x, world_pos.y, world_pos.z), Blocks::Ice | Blocks::PackedIce) {
            for player in world.players.values_mut() {
                player.write_packet(&SoundEffect {
                    sound: Sounds::GlassBreak.id(),
                    pos_x: world_pos.x as f64 + 0.5,
                    pos_y: world_pos.y as f64 + 0.5,
                    pos_z: world_pos.z as f64 + 0.5,
                    volume: 1.0,
                    pitch: 0.8,
                });
            }
        }
        world.set_block_at(Blocks::Air, world_pos.x, world_pos.y, world_pos.z);
    }
    for &(x, z) in OBSTACLE_PATTERNS[floor][pattern_index] {
        let world_pos = room.get_world_block_pos(&BlockPos::new(x, y + 1, z));
        world.set_block_at(Blocks::Air, world_pos.x, world_pos.y, world_pos.z);
    }
}

/// Called every tick from `Room::tick` for every room (a no-op for every room but the one whose
/// `ice_fill_state` is `Some`), same idiom `teleport_maze::tick` already established for this
/// exact style of walked-not-clicked puzzle.
pub fn tick(room: &mut Room, world: &mut World) {
    let Some(state_rc) = room.ice_fill_state.clone() else { return };

    // Extracted into a plain owned value (not matched directly against `state_rc.borrow()` in an
    // `if let`'s own condition) deliberately - `if let PAT = EXPR { BODY }` extends any temporary
    // created in `EXPR` (here, the `Ref` guard `.borrow()` returns) across the *entire* `BODY`,
    // not just the pattern match itself. `activate_floor` below needs its own `borrow_mut()` -
    // with the match written the other way, that outer `Ref` guard would still be alive when it
    // ran, panicking with "already borrowed". This `match` is instead a standalone expression
    // whose *value* is what `let` binds, so its own scrutinee borrow drops at the end of the
    // `match`, well before `activate_floor` is ever called.
    let respawning_at = match &state_rc.borrow().layer {
        LayerState::Respawning { resume_at_tick } => Some(*resume_at_tick),
        LayerState::Active { .. } => None,
        LayerState::Done => return,
    };

    if let Some(resume_at_tick) = respawning_at {
        if world.tick_count < resume_at_tick {
            return;
        }
        // `true` - this is always the post-fail-break restore here (a fresh floor's first
        // activation calls `activate_floor` directly, never through this respawn path), so the
        // Ice `break_floor` blanked away needs putting back too.
        activate_floor(&state_rc, room, world, true);
        return;
    }

    let client_ids: Vec<ClientId> = world.players.keys().copied().collect();
    for client_id in client_ids {
        tick_player(&state_rc, room, world, client_id);
    }
}

/// One player's movement check for the current tick - mirrors RustClear's own per-player loop
/// inside its `tick()` (adjacency/repeat validation, then success/fail handling), just split out
/// per-player instead of RustClear's single loop-with-early-break (this codebase's other
/// walked-puzzle, `teleport_maze.rs`, is likewise per-player).
fn tick_player(state_rc: &Rc<RefCell<IceFillState>>, room: &Room, world: &mut World, client_id: ClientId) {
    let Some(player) = world.players.get(&client_id) else { return };
    let feet_block = BlockPos::new(
        player.position.x.floor() as i32,
        (player.position.y - 0.1).floor() as i32,
        player.position.z.floor() as i32,
    );

    enum Outcome { None, Correct { done: bool, walked: Vec<BlockPos> }, Fail(&'static str) }

    let outcome = {
        let mut state = state_rc.borrow_mut();
        let LayerState::Active { footprint, remaining, tracked } = &mut state.layer else { return };

        if !footprint.contains(&feet_block) {
            return;
        }

        if let Some(&last) = tracked.get(&client_id) {
            if last == feet_block {
                Outcome::None
            } else {
                let dx = feet_block.x - last.x;
                let dz = feet_block.z - last.z;
                if dx != 0 && dz != 0 {
                    // A genuine diagonal step (both axes changed) - always a fail, regardless of
                    // distance, since it's the real corner-cutting cheese this check exists to
                    // catch: it'd let a player skip a tile without ever landing on it.
                    Outcome::Fail("§cDon't move diagonally! Bad!")
                } else {
                    // A straight-line move, possibly skipping several tiles in one tick (fast
                    // movement/speed effects/network jitter causing a tick to sample two positions
                    // further apart than 1 block - not the player's fault, and not a real skip
                    // since every tile in between still gets walked over physically). Walk every
                    // intermediate tile along that line and mark each visited, rather than only
                    // checking the final landing tile - this used to `Fail` on any two-tile gap,
                    // misreporting ordinary fast walking as "diagonal" or "wrong block".
                    let steps = dx.abs().max(dz.abs());
                    let step_x = dx.signum();
                    let step_z = dz.signum();
                    let mut bad_tile = None;
                    let mut walked = Vec::with_capacity(steps as usize);
                    for i in 1..=steps {
                        let pos = BlockPos::new(last.x + step_x * i, last.y, last.z + step_z * i);
                        if !footprint.contains(&pos) {
                            bad_tile = Some(());
                            break;
                        }
                        if remaining.remove(&pos) {
                            walked.push(pos); // only newly-visited tiles need converting to PackedIce
                        }
                    }
                    if bad_tile.is_some() {
                        Outcome::Fail("§cOops! You stepped on the wrong block!")
                    } else {
                        tracked.insert(client_id, feet_block);
                        Outcome::Correct { done: remaining.is_empty(), walked }
                    }
                }
            }
        } else if !remaining.remove(&feet_block) {
            Outcome::Fail("§cOops! You stepped on the wrong block!")
        } else {
            tracked.insert(client_id, feet_block);
            Outcome::Correct { done: remaining.is_empty(), walked: vec![feet_block] }
        }
    };

    match outcome {
        Outcome::None => {}
        Outcome::Correct { done, walked } => {
            // Per explicit request: only ever turn an actual Ice block into PackedIce - never
            // blindly overwrite whatever's currently there. In ordinary play these tiles are
            // always Ice at this point anyway (the `remaining` check above already refuses a
            // repeat visit), but this stays correct even if something odd left one in a different
            // state. `walked` covers every tile actually crossed this tick, not just the final
            // landing tile - see its own doc comment for why a fast straight-line move can cover
            // more than one.
            for pos in walked {
                if matches!(world.get_block_at(pos.x, pos.y, pos.z), Blocks::Ice) {
                    world.set_block_at(Blocks::PackedIce, pos.x, pos.y, pos.z);
                    // Per explicit request: a wool-break sound on every tile actually converted.
                    for player in world.players.values_mut() {
                        player.write_packet(&SoundEffect {
                            sound: Sounds::ClothBreak.id(),
                            pos_x: pos.x as f64 + 0.5,
                            pos_y: pos.y as f64 + 0.5,
                            pos_z: pos.z as f64 + 0.5,
                            volume: 1.0,
                            pitch: 0.8,
                        });
                    }
                }
            }
            if done {
                on_floor_cleared(state_rc, room, world);
            }
        }
        Outcome::Fail(message) => {
            if let Some(player) = world.players.get_mut(&client_id) {
                player.send_message(message);
            }
            on_floor_failed(state_rc, room, world);
        }
    }
}

/// A floor's last tile was just correctly stepped on - ascending harp chime (pitch rising with
/// floor number, matching RustClear's own `current_layer as f32 * 0.2`), then either the next
/// floor or full completion.
fn on_floor_cleared(state_rc: &Rc<RefCell<IceFillState>>, room: &Room, world: &mut World) {
    let floor = state_rc.borrow().current_floor;
    let pitch_base = floor as f32 * 0.2;
    for (i, extra) in [0.0_f32, 0.1, 0.2].into_iter().enumerate() {
        let world_tick = world.tick_count;
        for player in world.players.values_mut() {
            let pos = player.position;
            world.scheduled_fixed_sounds.push(crate::server::world::tactical_insertion::ScheduledFixedSound {
                due_tick: world_tick + (i as u64 * 5),
                sound: Sounds::Harp,
                volume: 1.0,
                pitch: 1.2 + pitch_base + extra,
                pos_x: pos.x,
                pos_y: pos.y,
                pos_z: pos.z,
            });
        }
    }

    let next_floor = floor + 1;
    if next_floor == 3 {
        // Frozen for good the instant the 3rd floor's last tile lands - see `LayerState::Done`'s
        // doc comment for the bug this prevents.
        state_rc.borrow_mut().layer = LayerState::Done;
        open_reward_chests(state_rc, room, world);
    } else {
        state_rc.borrow_mut().current_floor = next_floor;
        // `false` - the next floor's own Ice is already there in the room's captured data,
        // never having been broken, same as the very first floor's own activation.
        activate_floor(state_rc, room, world, false);
    }
}

/// A wrong step or diagonal move - per explicit request, breaks every Ice tile and obstacle block
/// on the current floor (`break_floor`) rather than just undoing whatever progress was made, then
/// schedules the same pattern's restore (`activate_floor(..., true)`, via `tick()`) after
/// `FAIL_BREAK_TICKS`.
fn on_floor_failed(state_rc: &Rc<RefCell<IceFillState>>, room: &Room, world: &mut World) {
    let (floor, pattern_index) = {
        let state = state_rc.borrow();
        (state.current_floor, state.obstacle_choice[state.current_floor])
    };
    // Per explicit request: no separate fail "sizzle" - `break_floor` above already plays a glass-
    // break sound per broken tile, and that's the only sound a fail should make.
    break_floor(room, world, floor, pattern_index);
    let resume_at_tick = world.tick_count + FAIL_BREAK_TICKS;
    state_rc.borrow_mut().layer = LayerState::Respawning { resume_at_tick };
}

/// All 3 floors cleared - opens the gate (see `GATE_MIN`/`MAX`'s doc comment for why this is a
/// plain clear rather than RustClear's own animated iron-bars-sliding-down effect) and makes the
/// two blessing chests behind it openable. Per explicit request, opening *both* chests - not
/// floor-clearing itself - is the actual win condition, so `room.puzzle_completed`/the "PUZZLE
/// SOLVED" broadcast happen later, over in `block_interact_action.rs`'s `Chest` handler, the first
/// time the second of the two is opened (see `DungeonSecret::puzzle_chest_group`).
fn open_reward_chests(state_rc: &Rc<RefCell<IceFillState>>, room: &Room, world: &mut World) {
    for x in GATE_MIN.x..=GATE_MAX.x {
        for y in GATE_MIN.y..=GATE_MAX.y {
            let pos = room.get_world_block_pos(&BlockPos::new(x, y, GATE_MIN.z));
            world.set_block_at(Blocks::Air, pos.x, pos.y, pos.z);
        }
    }

    let door_sound_pos = room.get_world_block_pos(&BlockPos::new(14, 74, 27));
    let pos = DVec3::new(door_sound_pos.x as f64 + 0.5, door_sound_pos.y as f64, door_sound_pos.z as f64 + 0.5);
    for player in world.players.values_mut() {
        player.write_packet(&SoundEffect {
            sound: Sounds::RandomWoodClick.id(),
            pos_x: pos.x,
            pos_y: pos.y,
            pos_z: pos.z,
            volume: 2.0,
            pitch: 0.5,
        });
    }

    let room_index = state_rc.borrow().room_index;
    // Shared between both chests: `puzzle_room_index` only actually fires the completion/broadcast
    // once this counter (starting at 2) reaches 0.
    let group_counter = Rc::new(Cell::new(CHEST_POSITIONS.len() as u8));
    for &rel_pos in &CHEST_POSITIONS {
        let world_pos = room.get_world_block_pos(&rel_pos);
        let mut secret = DungeonSecret::new(SecretType::Chest { direction: Direction::North.rotate(room.rotation) }, world_pos, 0.0);
        // Already sitting in the world as part of the room's own static data - there's no AABB-
        // proximity reveal step to wait for (unlike an ordinary secret chest), so this is marked
        // spawned up front and `spawn_into_world` is called directly below instead of through
        // `secrets::tick`'s usual has_spawned/AABB check.
        secret.has_spawned = true;
        secret.blessing_texture = Some(REWARD_BLESSING_TEXTURE);
        secret.puzzle_room_index = Some(room_index);
        secret.puzzle_chest_group = Some(group_counter.clone());
        // Doesn't bump the room's found_secrets tally - same reasoning as Tic Tac Toe's reward
        // chest (see `DungeonSecret::counts_as_secret`'s doc comment): this is the puzzle's own
        // win condition, not one of its counted secrets.
        secret.counts_as_secret = false;
        let secret_rc = Rc::new(RefCell::new(secret));
        let secret_mut = secret_rc.borrow_mut();
        DungeonSecret::spawn_into_world(&secret_rc, secret_mut, world);
    }
}

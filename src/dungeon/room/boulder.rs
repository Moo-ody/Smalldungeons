//! Boulder puzzle - real name "Box" (confirmed via a Hypixel Forums puzzle guide: "the goal is
//! to reach the chest in the back through a mess of boxes... moved by clicking the buttons on
//! their sides"), not a rolling ball despite the room's internal name. A Sokoban-style push
//! puzzle: 3x3x3 boxes sit on a 7x6 grid (3-block spacing), each pushable exactly one grid cell
//! (3 blocks) via a button or its face-center sign. Per direct in-game observation (not guessed):
//! pushing a box into a cell occupied by a box of the SAME block type is blocked outright; a
//! DIFFERENT block type there just gets silently overwritten/destroyed instead of blocking the
//! push - collision is keyed on block type, not "is this cell occupied".
//!
//! There are 8 known real random layouts (confirmed directly), captured live via the
//! `RoomsWorldHelper`-equivalent `/boulderpuzzle` ZJS module (reaching into Odin's own room
//! detection - see that module's own doc comments in the MIO CHEAT INSTANCE's `meow123` module)
//! into `room_data/misc/puzzles/Boulder/boulder_1.json` .. `boulder_8.json`. Those raw captures
//! are per-block dumps of the whole scanned area (walls, floor, decoration included, not just the
//! boxes) - `convert_boulder_patterns` (the `#[ignore]`d test at the bottom of this file) reduces
//! each one down to just its occupied grid cells' box structure + trigger positions into
//! `patterns.json`, which is what actually gets embedded/read at runtime.
//!
//! The reward chest is real, static, and already part of the room's own captured architecture at
//! room-relative `(15, 66, 29)` (confirmed by decoding the recaptured `83,boulder,-60,-564.json`
//! room data directly) - unlike Ice Fill's blessing chests, it needs no completion gating of its
//! own: the boxes are themselves the only thing physically blocking a path to it, so correctly
//! solving the push puzzle is exactly what makes it reachable. Opening it marks the puzzle solved,
//! the same single-chest `DungeonSecret::puzzle_room_index` mechanism Teleport Maze's own reward
//! chest already uses.
//!
//! Position math note: unlike a puzzle whose interactable positions never move (Tic Tac Toe's
//! buttons, Creeper Beams' lanterns), a box's trigger positions relocate every time it's pushed,
//! so `handle_push` (reached only from a `BlockInteractAction`, with just a `Player` - no `Room`
//! in scope, matching every other puzzle's own interact handler) needs to redo the room-relative
//! -> world conversion itself. Rather than plumb `&Room` all the way from the interact handler
//! (awkward: `player.server_mut()` already holds the only path to both `dungeon.rooms` and
//! `world`, and borrowing a room immutably while also borrowing `world` mutably through the same
//! `Server` doesn't split via safe field access alone), `BoulderState` just stores the room's own
//! `rotation`/corner once at setup - both are fixed for the room's entire lifetime - and
//! `world_pos_for` below reproduces `Room::get_world_block_pos`'s exact formula from those stored
//! values instead of a live `&Room`.

use crate::dungeon::room::room::Room;
use crate::dungeon::room::secrets::{DungeonSecret, SecretType};
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::player::player::{ClientId, Player};
use crate::server::utils::direction::Direction;
use crate::server::world::World;
use crate::utils::seeded_rng::seeded_rng;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The 7x6 box grid's cell centers (3-block spacing) - confirmed real via Odin's own
/// `BoulderSolver.kt` sensor scan (`x in 24 downTo 6 step 3`, `z in 24 downTo 9 step 3`).
const GRID_XS: [i32; 7] = [6, 9, 12, 15, 18, 21, 24];
const GRID_ZS: [i32; 6] = [9, 12, 15, 18, 21, 24];
/// A box spans this far from its grid cell's center on every horizontal axis, and this exact Y
/// range - confirmed via the live capture's own majority-vote grid reconstruction.
const BOX_RADIUS: i32 = 1;
const BOX_Y_MIN: i32 = 64;

/// Real, static, already part of the room's own architecture - see the module doc comment.
const REWARD_CHEST_POS: BlockPos = BlockPos { x: 15, y: 66, z: 29 };

/// The real captured blessing skull texture - same constant (by value) as `tic_tac_toe.rs`'s
/// `REWARD_BLESSING_TEXTURE`, `ice_path.rs`'s `REWARD_BLESSING_TEXTURE`, and `teleport_maze.rs`'s
/// `SOLVE_BLESSING_TEXTURE` - each file keeps its own copy rather than importing a shared one. Per
/// explicit request, Boulder's own reward chest is a blessing chest too.
const REWARD_BLESSING_TEXTURE: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZTkzZTIwNjg2MTc4NzJjNTQyZWNkYTFkMjdkZjRlY2U5MWM2OTk5MDdiZjMyN2M0ZGRiODUzMDk0MTJkMzkzOSJ9fX0=";

/// How long (ticks) a floor tile stays lit as white stained glass after last being activated
/// before reverting to an invisible barrier - see `tick`'s own doc comment for the full mechanic.
const FLOOR_GLASS_LIFETIME_TICKS: u64 = 10;

fn grid_cells() -> impl Iterator<Item = (i32, i32)> {
    GRID_XS.iter().flat_map(|&x| GRID_ZS.iter().map(move |&z| (x, z)))
}

fn is_valid_cell(x: i32, z: i32) -> bool {
    GRID_XS.contains(&x) && GRID_ZS.contains(&z)
}

/// Reproduces `Room::get_world_block_pos` from a stored rotation/corner instead of a live `&Room`
/// - see the module doc comment for why `handle_push` can't just hold onto a `&Room`.
fn world_pos_for(rotation: Direction, corner: BlockPos, local: BlockPos) -> BlockPos {
    local.rotate(rotation).add_x(corner.x).add_z(corner.z)
}

/// Neither `Direction` nor `Blocks` derive `Serialize`/`Deserialize` (both are simple/macro-
/// generated types not worth adding a serde dependency to just for this one puzzle's data file),
/// so `TriggerSpec`/`BoxTemplate` store a plain lowercase direction name and a raw
/// `get_block_state_id()` value instead, converted at the edges by `direction_from_str`/
/// `direction_to_str` and `Blocks::from`/`Blocks::get_block_state_id`.
fn direction_from_str(facing: &str) -> Direction {
    match facing {
        "north" => Direction::North,
        "south" => Direction::South,
        "east" => Direction::East,
        "west" => Direction::West,
        "up" => Direction::Up,
        _ => Direction::Down,
    }
}

fn direction_to_str(direction: Direction) -> &'static str {
    match direction {
        Direction::North => "north",
        Direction::South => "south",
        Direction::East => "east",
        Direction::West => "west",
        Direction::Up => "up",
        Direction::Down => "down",
    }
}

/// A button or sign mounted on a box - found by inverting its own facing (a sign facing south is
/// mounted on the block immediately to ITS north, i.e. `attached_to = sign_pos + opposite(facing)`
/// - not by which fixed-radius window it happens to fall in. Confirmed necessary live: a sign's
/// real position can be one block beyond the box's own 3x3x3 core (mounted flush on its outer
/// face), which a naive "must be within the box's own +-1 volume" scan misses entirely - the
/// exact bug behind boxes visibly leaving their attached signs behind when pushed. `dx`/`dy`/`dz`
/// are therefore not clamped to +-1 the way the box's own core cells are.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Attachment {
    /// Offset from the box's own grid center to the sign/button's own position (not the position
    /// it's attached to) - may have a magnitude of 2 on the axis it juts out past the core.
    dx: i32,
    dy: i32,
    dz: i32,
    /// The exact block-state id (preserves the sign/button's own facing) - see the module-level
    /// doc comment for why this is a raw id, not `Blocks`.
    block: u16,
    /// `true` for every button (clickable anywhere on a face) and every sign positioned at the
    /// exact horizontal center of a face (never top/bottom-center, never a corner/edge) - most
    /// captured signs are just decorative perimeter-wall dressing sharing the same block type as
    /// a real trigger, so position is what actually distinguishes the two, not the block type.
    is_trigger: bool,
    /// The direction pushing this attachment (when `is_trigger`) sends the box - equal to its own
    /// (already room-rotation-canonicalized) facing. See the module-level doc comment above for
    /// why this is a plain string, not `Direction`. Meaningless when `is_trigger` is false.
    direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BoxTemplate {
    grid_x: i32,
    grid_z: i32,
    /// The box's solid 3x3x3 core ONLY (plain material blocks - never a sign/button, those are
    /// `attachments` instead, since they can extend past this volume) - hex `block_data`, same
    /// convention as this project's own `RoomData::block_data`, laid out dx fastest / dz next /
    /// dy slowest over the fixed 3x3x3 box centered at its grid cell.
    block_data: String,
    /// The block-state id (`Blocks::get_block_state_id()`) used for the same-type-blocks-a-push
    /// collision rule - the box's single most common core block (its wood type). See the
    /// module-level doc comment for why this is a raw id, not `Blocks`.
    material: u16,
    attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PatternTemplate {
    boxes: Vec<BoxTemplate>,
}

const PATTERNS_JSON: &str = include_str!("../../room_data/misc/puzzles/Boulder/patterns.json");

fn load_patterns() -> Vec<PatternTemplate> {
    serde_json::from_str(PATTERNS_JSON).expect("boulder patterns.json should always parse - generated by convert_boulder_patterns")
}

fn decode_box_block_data(hex: &str) -> [Blocks; 27] {
    let mut cells = [Blocks::Air; 27];
    for (i, cell) in cells.iter_mut().enumerate() {
        let Some(hex_str) = hex.get(i * 4..i * 4 + 4) else { break };
        let Ok(id) = u16::from_str_radix(hex_str, 16) else { continue };
        *cell = Blocks::from(id);
    }
    cells
}

fn box_local_index(dx: i32, dy: i32, dz: i32) -> usize {
    ((dx + BOX_RADIUS) + (dz + BOX_RADIUS) * 3 + (dy + BOX_RADIUS) * 9) as usize
}

#[derive(Debug)]
pub struct BoulderState {
    room_index: usize,
    rotation: Direction,
    corner: BlockPos,
    /// Keyed by grid cell - absent means that cell is currently empty (no box there, whether it
    /// started that way or got pushed away).
    boxes: HashMap<(i32, i32), BoxTemplate>,
    /// World positions currently lit as white stained glass (the floor light-up effect - see
    /// `tick`'s own doc comment), mapped to the tick they revert back to an invisible barrier at
    /// if not re-activated before then.
    active_floor_tiles: HashMap<BlockPos, u64>,
}

/// Sets up the Boulder puzzle for `room` if it actually is one - picks one of the 8 real captured
/// layouts at random (fixed for the whole dungeon, matching every other puzzle here with a
/// randomly-chosen-once layout, e.g. Ice Fill's obstacle patterns), places its boxes, registers
/// every button/sign trigger, and registers the room's own real reward chest. No-op for every
/// other room. Called once from `Room::load_into_world`, after the room's own static blocks are
/// placed (same ordering the earlier test overlay needed to actually override anything).
pub fn setup(room: &mut Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Boulder" {
        return;
    }

    let patterns = load_patterns();
    let pattern_index = rand::Rng::random_range(&mut seeded_rng(), 0..patterns.len());
    let pattern = &patterns[pattern_index];

    let rotation = room.rotation;
    let corner = room.get_corner_pos();

    let boxes: HashMap<(i32, i32), BoxTemplate> = pattern.boxes.iter()
        .map(|template| ((template.grid_x, template.grid_z), template.clone()))
        .collect();
    let state_rc = Rc::new(RefCell::new(BoulderState { room_index, rotation, corner, boxes, active_floor_tiles: HashMap::new() }));
    rebuild_all(rotation, corner, world, &state_rc.borrow(), &state_rc);
    room.boulder_state = Some(state_rc);

    // The real reward chest - already sitting in the room's own static data, so this only needs
    // to register it as interactable, not place it. Per the module doc comment, no completion
    // gate is needed: the boxes are the only thing physically blocking it, so there's nothing
    // else to check before it should be openable. Per explicit request, this is a blessing chest
    // (the same real captured effect Teleport Maze/Ice Path/Tic Tac Toe's own reward chests use).
    let world_pos = world_pos_for(rotation, corner, REWARD_CHEST_POS);
    let mut secret = DungeonSecret::new(SecretType::Chest { direction: Direction::North.rotate(rotation) }, world_pos, 0.0);
    secret.blessing_texture = Some(REWARD_BLESSING_TEXTURE);
    secret.has_spawned = true;
    secret.puzzle_room_index = Some(room_index);
    secret.counts_as_secret = false;
    let secret_rc = Rc::new(RefCell::new(secret));
    let secret_mut = secret_rc.borrow_mut();
    DungeonSecret::spawn_into_world(&secret_rc, secret_mut, world);
}

/// Boulder's floor light-up effect - per explicit request: standing on a barrier block reveals a
/// 3x3 patch of white stained glass centered on the tile directly under your feet (one block out
/// in every horizontal direction, same Y), and each tile in that patch reverts back to an
/// invisible barrier `FLOOR_GLASS_LIFETIME_TICKS` after it was last activated. Activation isn't
/// limited to the exact tile you're standing on - it's *any* barrier block within that 3x3, at
/// your own feet's Y level specifically, whether or not the tile you're actually standing on is
/// itself a barrier (confirmed real: barrier blocks one tile away from you activate too, as long
/// as they're at your feet's height). Called every tick for every room from `Room::tick` - a no-op
/// for every room but the one with an active `boulder_state`.
pub fn tick(room: &Room, world: &mut World) {
    if room.room_data.name != "Boulder" {
        return;
    }
    let Some(state_rc) = room.boulder_state.clone() else { return };
    let mut state = state_rc.borrow_mut();
    let current_tick = world.tick_count;
    let (min_bound, max_bound) = room.get_world_bounds();

    let client_ids: Vec<ClientId> = world.players.keys().copied().collect();
    for client_id in client_ids {
        let Some(player) = world.players.get(&client_id) else { continue };
        let feet_x = player.position.x.floor() as i32;
        let feet_y = (player.position.y - 0.1).floor() as i32;
        let feet_z = player.position.z.floor() as i32;

        // Restrict to players actually standing inside THIS room - `world.get_block_at`/
        // `set_block_at` below operate on absolute world positions with no room affiliation of
        // their own, so without this check a barrier anywhere else in the dungeon (a falling
        // floor tile, a wither door) that happened to be under some other player's feet at the
        // same moment this Boulder room ticks would get swapped to glass too. Per explicit
        // correction: this effect is a Boulder-puzzle-only mechanic, never a generic
        // "any barrier under your feet" one.
        if feet_x < min_bound.x || feet_x > max_bound.x
            || feet_y < min_bound.y || feet_y > max_bound.y
            || feet_z < min_bound.z || feet_z > max_bound.z {
            continue;
        }

        for dx in -1..=1 {
            for dz in -1..=1 {
                let pos = BlockPos::new(feet_x + dx, feet_y, feet_z + dz);
                let current = world.get_block_at(pos.x, pos.y, pos.z);
                if matches!(current, Blocks::Barrier) {
                    world.set_block_at(Blocks::StainedGlass { color: 0 }, pos.x, pos.y, pos.z);
                } else if !matches!(current, Blocks::StainedGlass { color: 0 }) {
                    continue; // neither a barrier nor already-lit glass - not this effect's tile
                }
                state.active_floor_tiles.insert(pos, current_tick + FLOOR_GLASS_LIFETIME_TICKS);
            }
        }
    }

    let expired: Vec<BlockPos> = state.active_floor_tiles.iter()
        .filter(|&(_, &revert_at)| current_tick >= revert_at)
        .map(|(&pos, _)| pos)
        .collect();
    for pos in expired {
        world.set_block_at(Blocks::Barrier, pos.x, pos.y, pos.z);
        state.active_floor_tiles.remove(&pos);
    }
}

/// Every world-independent (pre-rotation, local) position ANY box's solid 3x3x3 core currently
/// occupies (only the cells that actually decode to a real, non-air block - a box's own core is
/// naturally already air wherever one of its own attachments sits, since pass 1 of
/// `convert_boulder_patterns` never lets a sign/button into `cells` to begin with, so this needs
/// no "exclude self" handling). Solid cores always win over an attachment sharing that position -
/// per explicit request, a box's own signs/buttons are never destroyed by this, just hidden for as
/// long as some other box's core covers that exact spot, and shown again the moment it doesn't
/// (`rebuild_all` recomputes this fresh from scratch on every change, so "remembering" a covered
/// attachment needs no extra bookkeeping - its data was never touched, only whether it's currently
/// drawn).
fn all_solid_core_positions<'a>(boxes: impl Iterator<Item = &'a BoxTemplate>) -> std::collections::HashSet<(i32, i32, i32)> {
    let mut positions = std::collections::HashSet::new();
    for template in boxes {
        let cells = decode_box_block_data(&template.block_data);
        for dx in -BOX_RADIUS..=BOX_RADIUS {
            for dy in -BOX_RADIUS..=BOX_RADIUS {
                for dz in -BOX_RADIUS..=BOX_RADIUS {
                    if cells[box_local_index(dx, dy, dz)] != Blocks::Air {
                        positions.insert((template.grid_x + dx, BOX_Y_MIN + BOX_RADIUS + dy, template.grid_z + dz));
                    }
                }
            }
        }
    }
    positions
}

/// Clears every currently-alive box's full footprint (core + attachments) to air and unregisters
/// every trigger - called on the *old* box set, before `state.boxes` gets mutated for a push, so
/// `rebuild_all` afterward starts from a clean slate rather than leaking stale
/// `world.interactable_blocks` entries or leftover blocks at positions no box occupies any more.
fn teardown_all(rotation: Direction, corner: BlockPos, world: &mut World, state: &BoulderState) {
    for template in state.boxes.values() {
        for dx in -BOX_RADIUS..=BOX_RADIUS {
            for dy in -BOX_RADIUS..=BOX_RADIUS {
                for dz in -BOX_RADIUS..=BOX_RADIUS {
                    let local = BlockPos::new(template.grid_x + dx, BOX_Y_MIN + BOX_RADIUS + dy, template.grid_z + dz);
                    let world_pos = world_pos_for(rotation, corner, local);
                    world.set_block_at(Blocks::Air, world_pos.x, world_pos.y, world_pos.z);
                }
            }
        }
        for attachment in &template.attachments {
            let local = BlockPos::new(template.grid_x + attachment.dx, BOX_Y_MIN + BOX_RADIUS + attachment.dy, template.grid_z + attachment.dz);
            let world_pos = world_pos_for(rotation, corner, local);
            world.set_block_at(Blocks::Air, world_pos.x, world_pos.y, world_pos.z);
            if attachment.is_trigger {
                world.interactable_blocks.remove(&world_pos);
            }
        }
    }
}

/// Wipes and re-places every currently-alive box's full footprint (core + attachments) and every
/// trigger registration, from scratch, based purely on `state.boxes`' current contents - the only
/// place any of this ever gets written, for both the very first placement (`setup`) and after
/// every push (`handle_push`). A full rebuild rather than an incremental patch specifically
/// because a push can both cover an attachment (a box's core lands where a neighbor's sign/button
/// used to show) and uncover one (a box that used to cover a neighbor moves away) in the same
/// move, and re-deriving "what does every position look like right now" from the complete current
/// state is far simpler than tracking those transitions - see `all_solid_core_positions`'s own
/// doc comment for the actual priority rule (solid cores always win).
///
/// Always paired with `teardown_all` called on the *old* box set first (`handle_push`'s job -
/// `setup` has no old set to tear down) - this only ever places/registers, it never removes
/// anything itself, so calling it without tearing down the previous state first would leak stale
/// `world.interactable_blocks` entries at positions that no longer belong to any box.
fn rebuild_all(rotation: Direction, corner: BlockPos, world: &mut World, state: &BoulderState, state_rc: &Rc<RefCell<BoulderState>>) {
    let solid = all_solid_core_positions(state.boxes.values());

    // Cores first, unconditionally - they always win, per explicit request.
    for template in state.boxes.values() {
        let cells = decode_box_block_data(&template.block_data);
        for dx in -BOX_RADIUS..=BOX_RADIUS {
            for dy in -BOX_RADIUS..=BOX_RADIUS {
                for dz in -BOX_RADIUS..=BOX_RADIUS {
                    let mut block = cells[box_local_index(dx, dy, dz)];
                    block.rotate(rotation);
                    let local = BlockPos::new(template.grid_x + dx, BOX_Y_MIN + BOX_RADIUS + dy, template.grid_z + dz);
                    let world_pos = world_pos_for(rotation, corner, local);
                    world.set_block_at(block, world_pos.x, world_pos.y, world_pos.z);
                }
            }
        }
    }

    // Then attachments - only drawn (and only clickable) where no core claims that exact spot.
    for template in state.boxes.values() {
        for attachment in &template.attachments {
            let local = BlockPos::new(template.grid_x + attachment.dx, BOX_Y_MIN + BOX_RADIUS + attachment.dy, template.grid_z + attachment.dz);
            let key = (local.x, local.y, local.z);
            let world_pos = world_pos_for(rotation, corner, local);

            if solid.contains(&key) {
                continue; // covered by some box's solid core - stays hidden, not destroyed
            }

            let mut block = Blocks::from(attachment.block);
            block.rotate(rotation);
            world.set_block_at(block, world_pos.x, world_pos.y, world_pos.z);

            if attachment.is_trigger {
                world.interactable_blocks.insert(world_pos, BlockInteractAction::BoulderTrigger {
                    state: state_rc.clone(),
                    grid_x: template.grid_x,
                    grid_z: template.grid_z,
                    // A button/sign's own `facing` points outward, toward whoever's standing
                    // there to click it - the push it sends the box travels the opposite way
                    // (deeper into the grid, away from the face you're touching), confirmed live:
                    // pushes were landing exactly backwards using the raw facing direction.
                    direction: direction_from_str(&attachment.direction).opposite(),
                });
            }
        }
    }
}

/// A box's core material - its single most common block within its solid 3x3x3 core (used only
/// for the same-type-blocks-a-push collision rule, not for placement; never a sign/button, those
/// are never part of `cells` any more - see `Attachment`'s own doc comment). Linear scan, not a
/// `HashMap<Blocks, _>` - `Blocks` doesn't derive `Hash`.
fn box_material(cells: &[Blocks; 27]) -> Blocks {
    let mut counts: Vec<(Blocks, usize)> = Vec::new();
    for block in cells {
        if *block == Blocks::Air {
            continue;
        }
        if let Some(entry) = counts.iter_mut().find(|(b, _)| b == block) {
            entry.1 += 1;
        } else {
            counts.push((*block, 1));
        }
    }
    counts.into_iter().max_by_key(|(_, count)| *count).map(|(block, _)| block).unwrap_or(Blocks::Air)
}

/// A trigger (button or qualifying face-center sign) was clicked - pushes the box at
/// `(grid_x, grid_z)` one grid cell (3 blocks) in `direction`, per the real, directly-observed
/// mechanic: blocked outright if the destination already holds a box of the exact same material;
/// otherwise the push succeeds, silently destroying whatever different-type box was there.
pub fn handle_push(player: &mut Player, state_rc: &Rc<RefCell<BoulderState>>, grid_x: i32, grid_z: i32, direction: Direction) {
    let (offset_x, _, offset_z) = direction.get_offset();
    let dest_x = grid_x + offset_x * 3;
    let dest_z = grid_z + offset_z * 3;
    if !is_valid_cell(dest_x, dest_z) {
        return; // pushing off the edge of the grid entirely - no-op
    }

    let mut state = state_rc.borrow_mut();
    let rotation = state.rotation;
    let corner = state.corner;

    let Some(moving) = state.boxes.get(&(grid_x, grid_z)).cloned() else { return };

    if let Some(blocker) = state.boxes.get(&(dest_x, dest_z)) {
        if blocker.material == moving.material {
            return; // same material - blocked outright, per the real mechanic
        }
    }

    let world = player.world_mut();

    // Tear down the *old* configuration first - see `teardown_all`'s own doc comment for why this
    // has to happen before `state.boxes` is mutated below, not after.
    teardown_all(rotation, corner, world, &state);

    // Destroy whatever different-type box (if any) already sat at the destination, then move.
    state.boxes.remove(&(dest_x, dest_z));
    state.boxes.remove(&(grid_x, grid_z));
    let mut moved = moving;
    moved.grid_x = dest_x;
    moved.grid_z = dest_z;
    state.boxes.insert((dest_x, dest_z), moved);

    // Rebuild from the new configuration - cores always win over any attachment sharing their
    // position (per explicit request), so a neighbor's sign/button that's now covered simply
    // isn't drawn/registered this pass, not destroyed - its data lives on in that box's own
    // (unchanged) template, ready to reappear the moment something uncovers it again.
    rebuild_all(rotation, corner, world, &state, state_rc);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::block::block_parameter::{ButtonDirection, StairDirection};

    /// Which raw block types actually count as a "qualifying" sign trigger position: horizontally
    /// centered on one axis (the other exactly 0) at the box's own middle Y layer - never
    /// top/bottom-center, never a corner/edge. Magnitude isn't restricted to the core's own
    /// surface (radius 1) - an attachment can sit one block beyond it (radius 2) and still be a
    /// real face-center trigger, see `Attachment`'s own doc comment. Most captured signs are just
    /// decorative perimeter-wall dressing sharing the same block type as a real trigger, so
    /// position is what actually distinguishes the two, not the block type.
    fn is_face_center(dx: i32, dy: i32, dz: i32) -> bool {
        dy == 0 && ((dx == 0 && dz != 0) || (dz == 0 && dx != 0))
    }

    fn direction_from_str(facing: &str) -> Direction {
        match facing {
            "north" => Direction::North,
            "south" => Direction::South,
            "east" => Direction::East,
            "west" => Direction::West,
            "up" => Direction::Up,
            _ => Direction::Down,
        }
    }

    fn button_direction_test(facing: &str, face: Option<&str>) -> ButtonDirection {
        use crate::server::block::metadata::BlockMetadata;
        let direction = match face {
            Some("FLOOR") => Direction::Up,
            Some("CEILING") => Direction::Down,
            _ => direction_from_str(facing),
        };
        ButtonDirection::from_meta(match direction {
            Direction::Down => 0,
            Direction::East => 1,
            Direction::West => 2,
            Direction::South => 3,
            Direction::North => 4,
            Direction::Up => 5,
        })
    }

    fn stair_direction_test(facing: &str) -> StairDirection {
        match facing {
            "east" => StairDirection::East,
            "west" => StairDirection::West,
            "south" => StairDirection::South,
            _ => StairDirection::North,
        }
    }

    /// Maps one captured (modern id + properties) entry to this project's `Blocks` enum - `None`
    /// for an unrecognized block type (left Air rather than guessed). Same mapping notes as the
    /// original test-overlay tool this puzzle replaces: 1.8's flattened-back planks/stone variant
    /// metadata, buttons' floor/ceiling folded into the same direction field as the 4 cardinals,
    /// stairs' facing meaning unchanged across versions.
    fn to_block(block_str: &str, props: &HashMap<String, String>) -> Option<Blocks> {
        let facing = || props.get("facing").map(String::as_str).unwrap_or("north");
        Some(match block_str {
            "minecraft:stone" => Blocks::Stone { variant: 0 },
            "minecraft:diorite" => Blocks::Stone { variant: 3 },
            "minecraft:andesite" => Blocks::Stone { variant: 5 },
            "minecraft:polished_andesite" => Blocks::Stone { variant: 6 },
            "minecraft:stone_bricks" => Blocks::StoneBrick { variant: 0 },
            "minecraft:chiseled_stone_bricks" => Blocks::StoneBrick { variant: 3 },
            "minecraft:birch_planks" => Blocks::WoodPlank { variant: 2 },
            "minecraft:jungle_planks" => Blocks::WoodPlank { variant: 3 },
            "minecraft:oak_wall_sign" => Blocks::WallSign { direction: direction_from_str(facing()) },
            "minecraft:stone_button" => Blocks::StoneButton {
                direction: button_direction_test(facing(), props.get("face").map(String::as_str)),
                powered: props.get("powered").map(String::as_str) == Some("true"),
            },
            "minecraft:stone_brick_stairs" => Blocks::StoneBrickStairs {
                direction: stair_direction_test(facing()),
                top_half: props.get("half").map(String::as_str) == Some("top"),
            },
            "minecraft:cobblestone_wall" => Blocks::CobblestoneWalls { variant: 0 },
            "minecraft:quartz_block" => Blocks::QuartzBlock { variant: 0 },
            "minecraft:hopper" => Blocks::Hopper {
                direction: direction_from_str(facing()),
                enabled: props.get("enabled").map(String::as_str) == Some("true"),
            },
            "minecraft:fire" => Blocks::Fire,
            "minecraft:barrier" => Blocks::Barrier,
            "minecraft:air" => Blocks::Air,
            _ => return None,
        })
    }

    /// Regenerates `patterns.json` from all 8 raw `/boulderpuzzle scan` captures. Not run by
    /// default - a one-off data-prep step, not a correctness test. Run with `cargo test
    /// convert_boulder_patterns -- --ignored --nocapture` after a fresh set of captures.
    #[test]
    #[ignore]
    fn convert_boulder_patterns() {
        #[derive(Deserialize)]
        struct CapturedBlock {
            x: i32,
            y: i32,
            z: i32,
            block: String,
            #[serde(default)]
            properties: HashMap<String, String>,
        }
        #[derive(Deserialize)]
        struct CapturedPattern {
            blocks: Vec<CapturedBlock>,
        }

        let raw_files: [&str; 8] = [
            include_str!("../../room_data/misc/puzzles/Boulder/boulder_1.json"),
            include_str!("../../room_data/misc/puzzles/Boulder/boulder_2.json"),
            include_str!("../../room_data/misc/puzzles/Boulder/boulder_3.json"),
            include_str!("../../room_data/misc/puzzles/Boulder/boulder_4.json"),
            include_str!("../../room_data/misc/puzzles/Boulder/boulder_5.json"),
            include_str!("../../room_data/misc/puzzles/Boulder/boulder_6.json"),
            include_str!("../../room_data/misc/puzzles/Boulder/boulder_7.json"),
            include_str!("../../room_data/misc/puzzles/Boulder/boulder_8.json"),
        ];

        let mut all_patterns = Vec::new();

        for raw in raw_files {
            let captured: CapturedPattern = serde_json::from_str(raw).expect("parse raw capture");

            let mut by_pos: HashMap<(i32, i32, i32), &CapturedBlock> = HashMap::new();
            for block in &captured.blocks {
                by_pos.insert((block.x, block.y, block.z), block);
            }

            // Pass 1: find every grid cell that's a real box - solid core material present in its
            // own 3x3x3 volume, never a sign/button (those are handled entirely in pass 2, since
            // a sign/button can extend past this volume - see `Attachment`'s own doc comment for
            // why that split is necessary, confirmed by real captured data: solid-core cells and
            // their attached signs sometimes fall in adjacent, not overlapping, grid windows).
            let mut cores: HashMap<(i32, i32), [Blocks; 27]> = HashMap::new();
            for (gx, gz) in grid_cells() {
                let mut cells = [Blocks::Air; 27];
                for dx in -BOX_RADIUS..=BOX_RADIUS {
                    for dy in -BOX_RADIUS..=BOX_RADIUS {
                        for dz in -BOX_RADIUS..=BOX_RADIUS {
                            let y = BOX_Y_MIN + BOX_RADIUS + dy;
                            let Some(captured_block) = by_pos.get(&(gx + dx, y, gz + dz)) else { continue };
                            let Some(block) = to_block(&captured_block.block, &captured_block.properties) else { continue };
                            if matches!(block, Blocks::WallSign { .. } | Blocks::StoneButton { .. }) {
                                continue; // never part of the core - always an Attachment instead
                            }
                            cells[box_local_index(dx, dy, dz)] = block;
                        }
                    }
                }
                if box_material(&cells) != Blocks::Air {
                    cores.insert((gx, gz), cells);
                }
            }

            // Pass 2: attach every captured sign/button to whichever real box it's actually
            // mounted on, found by inverting its own facing - not by which fixed-radius window it
            // happens to fall in (see `Attachment`'s own doc comment for why that distinction
            // matters: this is the fix for boxes visibly leaving their signs behind when pushed).
            let mut attachments: HashMap<(i32, i32), Vec<Attachment>> = HashMap::new();
            for captured_block in &captured.blocks {
                let Some(block) = to_block(&captured_block.block, &captured_block.properties) else { continue };
                if !matches!(block, Blocks::WallSign { .. } | Blocks::StoneButton { .. }) {
                    continue;
                }
                let Some(facing_str) = captured_block.properties.get("facing") else { continue };
                let facing = direction_from_str(facing_str);
                let (ox, oy, oz) = facing.opposite().get_offset();
                let attached_x = captured_block.x + ox;
                let attached_y = captured_block.y + oy;
                let attached_z = captured_block.z + oz;

                let Some((&(gx, gz), _)) = cores.iter().find(|(&(gx, gz), _)| {
                    (attached_x - gx).abs() <= BOX_RADIUS
                        && (attached_y - (BOX_Y_MIN + BOX_RADIUS)).abs() <= BOX_RADIUS
                        && (attached_z - gz).abs() <= BOX_RADIUS
                }) else {
                    continue; // not mounted on any real box's core - decorative room architecture
                };

                let dx = captured_block.x - gx;
                let dy = captured_block.y - (BOX_Y_MIN + BOX_RADIUS);
                let dz = captured_block.z - gz;
                let is_trigger = matches!(block, Blocks::StoneButton { .. }) || is_face_center(dx, dy, dz);

                attachments.entry((gx, gz)).or_default().push(Attachment {
                    dx,
                    dy,
                    dz,
                    block: block.get_block_state_id(),
                    is_trigger,
                    direction: facing_str.clone(),
                });
            }

            let mut boxes = Vec::new();
            for ((gx, gz), cells) in cores {
                let mut hex = String::with_capacity(27 * 4);
                for block in &cells {
                    hex.push_str(&format!("{:04x}", block.get_block_state_id()));
                }

                boxes.push(BoxTemplate {
                    grid_x: gx,
                    grid_z: gz,
                    block_data: hex,
                    material: box_material(&cells).get_block_state_id(),
                    attachments: attachments.remove(&(gx, gz)).unwrap_or_default(),
                });
            }

            all_patterns.push(PatternTemplate { boxes });
        }

        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/room_data/misc/puzzles/Boulder/patterns.json");
        let json = serde_json::to_string_pretty(&all_patterns).unwrap();
        std::fs::write(path, json).expect("write patterns.json");
        println!("wrote {path}");
    }
}

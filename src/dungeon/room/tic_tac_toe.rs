//! Tic Tac Toe puzzle - the board-display half of it (map generation, the 9 Item Frames, and
//! swapping a cell's shown X/O/blank). Researched, not guessed:
//!
//! - The Hypixel Wiki: a 3x3 board, played against an AI you must beat or tie (a loss fails the
//!   puzzle); standard strategy (take the center if the AI takes a corner and vice versa, block
//!   any two-in-a-row). Solving reveals a reward chest; there's also a *separate* secret chest
//!   behind a wall, opened by a lever reached via stairs at the back of the room.
//! - OdinClient's real, decompiled solver (`TTTSolver.kt`) confirms the actual implementation
//!   real Hypixel uses: the board is 9 real Item Frames, each holding a filled map - not colored
//!   wool or blocks. Odin detects X vs O by reading *one* pixel out of the map's 128x128 color
//!   array: index 8256 (row 64, column 64 - dead center). If that byte is exactly `114`, it's an
//!   X; if a map exists there but the byte differs, it's an O; no frame/map there at all is
//!   blank. Odin's own solver is just this detector plus a colored-box overlay - it never
//!   computes best moves, the player still has to actually play.
//! - This room's own captured layout (`24,tic_tac_toe,-96,-168.json`) decoded directly: 9 Stone
//!   Buttons in an exact 3x3 grid at room-relative `x=8`, `y` in `{70,71,72}`, `z` in
//!   `{15,16,17}` - these are what a player actually right-clicks to place a move - and one Lever
//!   at `(13, 73, 25)`, almost certainly the separate secret chest's.
//!
//! An unplayed cell is just its bare button - no frame exists there at all, matching Odin's own
//! solver (which only ever finds a frame where a move has actually been made; see `TTTSolver.kt`
//! above). The instant a cell is played (by the bot or a player), its button is destroyed and
//! replaced with a real Item Frame holding the mark's map - see `set_cell`.
//!
//! This file covers the display side confirmed above: the 3 reusable map designs (blank/X/O) -
//! X and O are real captured Hypixel map data (`room_data/misc/tictactoe/map_<id>.bin`, the exact
//! 128x128 palette-index array Hypixel's own server sent, byte-for-byte; the sibling `.json`
//! records which real in-game Item Frame entities each was pulled from), blank is still a plain
//! synthesized white canvas since real unplayed cells have no frame/map at all to capture -
//! sending them to the client once as real map data, and spawning/swapping the 9 Item Frames -
//! plus the actual game: the bot's opening move, the 9 buttons' click handling, a minimax AI, and
//! win/tie/loss detection.
//!
//! The bot's known real behavior (forum guides on beating/tying it, e.g. "place your O in any
//! corner" - implying the bot itself is X - describe it as coded to play perfectly and never
//! lose, so the only realistic outcomes against it are a tie or a loss; it's also documented as
//! usually opening in the middle on lower floors but in a corner on F3). This implementation's
//! bot always opens by taking a random corner as `X` the instant the room loads - before any
//! player has clicked anything - matching that "predictable pattern" corner behavior, then plays
//! a real minimax search (perfect play - never loses, so a tie is the best a player can force)
//! for every move after.

use crate::dungeon::door::{DoorEntityImpl, DOOR_ENTITY_OFFSET};
use crate::dungeon::room::room::Room;
use crate::net::protocol::play::clientbound::{Maps, SoundEffect};
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{EntityId, NoEntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::items::item_stack::ItemStack;
use crate::dungeon::room::secrets::{DungeonSecret, SecretType};
use crate::server::player::player::Player;
use crate::server::utils::direction::Direction;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::world::{ScheduledFixedSound, World};
use crate::server::block::rotatable::Rotatable;
use rand::Rng;
use std::cell::RefCell;
use std::rc::Rc;

/// One 128x128 map's worth of color-palette-index pixels.
const MAP_SIZE: usize = 128;

/// Vanilla's real map color palette is `base_color_id * 4 + shade`, `shade` in 0..4 (multipliers
/// 180/220/255/135 - shade 2 is the true, undarkened color). `SNOW` (id 8, RGB 255,255,255) at
/// shade 2 is a clean white - `8*4+2`.
const MAP_COLOR_WHITE: u8 = 8 * 4 + 2;
/// `RED` (id 28, RGB 153,51,51 - a real dark red, not the bright `TNT` red) at shade 2 - `28*4+2`
/// = **114**, which is not a coincidence: this is the exact value Odin's real `findSlotState`
/// checks the center pixel against to detect an X. Independently landing on the same value by
/// picking "dark red, undarkened shade" confirms it's the real color Hypixel's own X/O maps use.
const MAP_COLOR_RED: u8 = 28 * 4 + 2;

/// Stable map ids for the 3 reusable board designs - sent once (see `send_map_definitions`) and
/// referenced by every cell's held item afterward via `ItemStack.metadata`. `1` is already used
/// by this codebase's own dungeon HUD map (`Dungeon::tick`'s `Maps { id: 1, .. }`) - these are
/// deliberately far from it.
const MAP_ID_BLANK: i32 = 100;
const MAP_ID_X: i32 = 101;
const MAP_ID_O: i32 = 102;

/// Real vanilla item id for a (filled) map in 1.8 - same item as an empty map, distinguished
/// purely by a nonzero `metadata` (damage value) pointing at real map data.
const MAP_ITEM_ID: i16 = 358;

/// A solid white 128x128 canvas - the "blank" cell design. No frame exists at an unplayed cell in
/// the real puzzle (see the module doc comment), so there's no captured map to use here.
fn make_blank_map() -> Vec<u8> {
    vec![MAP_COLOR_WHITE; MAP_SIZE * MAP_SIZE]
}

/// Real captured Hypixel map data for the X/O board designs - see `room_data/misc/tictactoe/`.
/// Each `.bin` is the exact 128x128 palette-index array Hypixel's server sent for that map id,
/// byte-for-byte (the sibling `.json` records which real in-game Item Frame entities it was
/// pulled from). `114` at the dead-center pixel (index `64*128+64`) is Odin's real `TTTSolver`
/// X-detection byte (see `MAP_COLOR_RED`) - `REAL_X_MAP` hits it, `REAL_O_MAP` doesn't, exactly
/// matching real X/O.
const REAL_X_MAP: &[u8] = include_bytes!("../../room_data/misc/puzzles/map_30876.bin");
const REAL_O_MAP: &[u8] = include_bytes!("../../room_data/misc/puzzles/map_30877.bin");

/// The real captured X map - see `REAL_X_MAP`.
fn make_x_map() -> Vec<u8> {
    debug_assert_eq!(REAL_X_MAP.len(), MAP_SIZE * MAP_SIZE, "map_30876.bin should be a full 128x128 palette array");
    debug_assert_eq!(REAL_X_MAP[64 * MAP_SIZE + 64], MAP_COLOR_RED, "map_30876.bin's center pixel should be Odin's X-detection byte");
    REAL_X_MAP.to_vec()
}

/// The real captured O map - see `REAL_X_MAP`.
fn make_o_map() -> Vec<u8> {
    debug_assert_eq!(REAL_O_MAP.len(), MAP_SIZE * MAP_SIZE, "map_30877.bin should be a full 128x128 palette array");
    debug_assert_ne!(REAL_O_MAP[64 * MAP_SIZE + 64], MAP_COLOR_RED, "map_30877.bin's center pixel must differ from the X-detection byte to read as O");
    REAL_O_MAP.to_vec()
}

fn map_designs() -> [(i32, Vec<u8>); 3] {
    [(MAP_ID_BLANK, make_blank_map()), (MAP_ID_X, make_x_map()), (MAP_ID_O, make_o_map())]
}

/// Sends the 3 board designs to every currently connected player as real map data (`Maps`,
/// 0x34) - a full-canvas region update (`columns`/`rows` 128, offset 0,0) per id. Called once at
/// room setup, which at boot happens with zero players actually connected yet (`main.rs`'s
/// `populate_dungeon_world` doc comment: "used... at boot") - so this alone can't be relied on to
/// reach anyone. `send_map_definitions_to` (called on every player join, see `server.rs`) is what
/// actually covers the real case; this is kept too for `dungeon_switch::switch_dungeon`, which
/// rebuilds rooms while players are already connected.
pub fn send_map_definitions(world: &mut World) {
    for (id, map_data) in map_designs() {
        let packet = Maps { id, scale: 0, icons: vec![], columns: MAP_SIZE as u8, rows: MAP_SIZE as u8, x: 0, z: 0, map_data };
        for player in world.players.values_mut() {
            player.write_packet(&packet);
        }
    }
}

/// Sends the same 3 board designs to just `player` - the client caches map data by id regardless
/// of whether an item holding that id has been seen yet, so this is safe to call unconditionally
/// on every join even for a player who never sets foot in the Tic Tac Toe room. See `server.rs`'s
/// join-game handling, right after `sync_player_view`.
pub fn send_map_definitions_to(player: &mut Player) {
    for (id, map_data) in map_designs() {
        let packet = Maps { id, scale: 0, icons: vec![], columns: MAP_SIZE as u8, rows: MAP_SIZE as u8, x: 0, z: 0, map_data };
        player.write_packet(&packet);
    }
}

/// What a cell currently shows - maps directly to one of the 3 reusable map ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Blank,
    X,
    O,
}

impl CellState {
    fn map_id(self) -> i32 {
        match self {
            CellState::Blank => MAP_ID_BLANK,
            CellState::X => MAP_ID_X,
            CellState::O => MAP_ID_O,
        }
    }
}

/// Room-relative button positions, decoded directly from this room's own captured `block_data` -
/// see the module doc comment. Index order matches Odin's own board indexing (row-major, 0..9,
/// `row = i/3, column = i%3`). Each cell's Item Frame occupies this same block once played
/// (`set_cell` clears the button here to Air first, then spawns the frame at this exact
/// coordinate) - confirmed in-game to sit flush on the wall behind it.
const CELL_POSITIONS: [BlockPos; 9] = [
    BlockPos { x: 8, y: 72, z: 15 }, BlockPos { x: 8, y: 72, z: 16 }, BlockPos { x: 8, y: 72, z: 17 },
    BlockPos { x: 8, y: 71, z: 15 }, BlockPos { x: 8, y: 71, z: 16 }, BlockPos { x: 8, y: 71, z: 17 },
    BlockPos { x: 8, y: 70, z: 15 }, BlockPos { x: 8, y: 70, z: 16 }, BlockPos { x: 8, y: 70, z: 17 },
];

/// Which way the board's item frames face in room-local space, before the room's actual world
/// rotation is applied - ground truth decoded directly from this room's own captured
/// `block_data`: all 9 real buttons carry metadata `1`, which `ButtonDirection::from_meta`
/// (`block_parameter.rs`) maps to `Direction::East`, and the wall's Iron Blocks sit at local
/// `x=7` - one block *west* of the buttons at `x=8` - confirming the button (and therefore the
/// frame replacing it) really does point east, away from that wall into the room.
///
/// Only used as a fallback in `setup` if the actual placed button can't be read back for some
/// reason - `Blocks::rotate` (used to rotate the real button blocks into place - see `room.rs`'s
/// block-placement loop) is a separate, independent code path from `Direction::rotate` (what
/// this constant would otherwise be rotated with), and `ButtonDirection`'s own `Rotatable` impl
/// carries a `// todo: fix rotation, its still broken` note - so `setup` instead reads back
/// whatever direction the real, already-placed button ended up with and matches that exactly,
/// rather than risk disagreeing with it by recomputing the rotation independently here.
const CELL_FACING: Direction = Direction::East;

/// The 4 corner cells in `CELL_POSITIONS`' row-major indexing (`row = i/3, column = i%3`) -
/// `(0,0)`, `(0,2)`, `(2,0)`, `(2,2)`. The bot's opening move always lands on one of these.
const CORNERS: [usize; 4] = [0, 2, 6, 8];

/// The 5(tall)x3(wide) Iron Block wall behind the board, room-local space - one block *west* of
/// the buttons (`CELL_FACING`'s doc comment: buttons sit at `x=8`, the wall at `x=7`), spanning
/// the same 3 `z` columns as the buttons (`15..=17`) plus one extra row above and below their
/// `y` range (`69..=73`, vs. the buttons' `70..=72`). Solving the puzzle drops this whole wall
/// away (`drop_wall`) to reveal the reward chest behind it, the same falling-block technique a
/// Wither door opens with.
const WALL_POSITIONS: [BlockPos; 15] = [
    BlockPos { x: 7, y: 69, z: 15 }, BlockPos { x: 7, y: 69, z: 16 }, BlockPos { x: 7, y: 69, z: 17 },
    BlockPos { x: 7, y: 70, z: 15 }, BlockPos { x: 7, y: 70, z: 16 }, BlockPos { x: 7, y: 70, z: 17 },
    BlockPos { x: 7, y: 71, z: 15 }, BlockPos { x: 7, y: 71, z: 16 }, BlockPos { x: 7, y: 71, z: 17 },
    BlockPos { x: 7, y: 72, z: 15 }, BlockPos { x: 7, y: 72, z: 16 }, BlockPos { x: 7, y: 72, z: 17 },
    BlockPos { x: 7, y: 73, z: 15 }, BlockPos { x: 7, y: 73, z: 16 }, BlockPos { x: 7, y: 73, z: 17 },
];

/// Room-relative position of the puzzle's actual reward chest - a real Chest block (id 54)
/// already sitting in this room's own captured `block_data` at this exact spot, directly behind
/// the Iron Block wall (`WALL_POSITIONS`) so a player can't physically reach it until the wall
/// drops. Left completely unregistered (not a `world.interactable_blocks` entry, not a
/// `DungeonSecret`) until the puzzle is actually won - `reveal_reward_chest`, called from
/// `finish_if_over`'s win/tie branch, is what turns it into a real "blessing" chest, the same
/// reveal-on-solve pattern Teleport Maze's own reward chest uses
/// (`teleport_maze.rs::reveal_reward_chest`/`EXISTING_CHEST`).
const REWARD_CHEST_POS: BlockPos = BlockPos { x: 2, y: 70, z: 16 };

/// Same shared "blessing" skull skin Teleport Maze's own reward chest uses - see
/// `EssenceEntityImpl`'s doc comment (`secrets.rs`) for what setting `DungeonSecret::blessing_texture`
/// actually triggers on open (the ascending `note.harp` sequence + floating, spinning skull).
const REWARD_BLESSING_TEXTURE: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZTkzZTIwNjg2MTc4NzJjNTQyZWNkYTFkMjdkZjRlY2U5MWM2OTk5MDdiZjMyN2M0ZGRiODUzMDk0MTJkMzkzOSJ9fX0=";

#[derive(Debug)]
pub struct TicTacToeState {
    room_index: usize,
    /// `None` until that cell is actually played - the bare button is left showing until then.
    /// The frame is spawned lazily (first non-blank `set_cell`), matching Odin's own solver
    /// (which only ever finds a frame where a move has actually been made).
    cell_entities: [Option<EntityId>; 9],
    /// World-space center of each cell's Item Frame, resolved once in `setup` (room-rotation
    /// applied via `get_world_block_pos`) so `set_cell` can spawn a frame without needing a
    /// `&Room` of its own.
    cell_positions: [DVec3; 9],
    /// World-space position of each cell's button block, resolved once in `setup` - `set_cell`
    /// clears this to Air the instant that cell is actually played.
    cell_block_positions: [BlockPos; 9],
    /// World-space positions of the wall's 15 Iron Blocks - see `WALL_POSITIONS`. Resolved once
    /// in `setup`, same as `cell_block_positions`.
    wall_block_positions: [BlockPos; 15],
    /// World-space position of the reward chest - see `REWARD_CHEST_POS`. Resolved once in
    /// `setup`, same as `cell_block_positions`/`wall_block_positions`.
    reward_chest_pos: BlockPos,
    /// `CELL_FACING` rotated by the room's actual world rotation - see `CELL_FACING`'s doc
    /// comment. Resolved once in `setup`, same as `cell_positions`.
    facing: Direction,
    cell_states: [CellState; 9],
    /// Set once a winner or a tie is reached - `interact_cell` stops accepting clicks after.
    game_over: bool,
    /// The bot's already-computed next move, waiting for its own real tick to actually fire (see
    /// `BOT_MOVE_DELAY_TICKS`) - `None` whenever no move is pending (the common case). Computing
    /// the move immediately (in `interact_cell`) but only *placing* it once `tick` sees this fire
    /// keeps the bot's play perfect either way; only the reveal is delayed.
    pending_bot_move: Option<PendingBotMove>,
}

/// See `TicTacToeState::pending_bot_move`. `username` is captured at the moment the player's own
/// move is made (not looked up again later) so the eventual PUZZLE SOLVED/FAIL message can still
/// credit them correctly from `tick`, which only ever has a `&mut World`, no specific `Player`.
#[derive(Debug, Clone)]
struct PendingBotMove {
    index: usize,
    fire_tick: u64,
    username: String,
}

/// How long the bot waits after the player's own move before actually placing its response -
/// per explicit request, always exactly 3 real seconds, not proportional to search time (the
/// minimax search itself is instant - see `best_bot_move`'s own doc comment).
const BOT_MOVE_DELAY_TICKS: u64 = 3 * 20;

/// Prepares the Tic Tac Toe board for `room` if it actually is one - no-op for every other room.
/// Called once from `Room::load_into_world`, **after** its block-placement loop has actually put
/// the 9 real buttons into the world (see the call site's doc comment) - `setup` reads one of
/// them back to learn the frames' rotation-correct facing, rather than recomputing that rotation
/// independently and risking disagreeing with it (see `CELL_FACING`'s doc comment).
///
/// Sends the 3 map designs, resolves each cell's world position/button position, registers the 9
/// buttons as interactable, then immediately plays the bot's opening move: a random corner as `X`
/// (see the module doc comment and `CORNERS`), before any player has had a chance to click
/// anything. Every other cell starts as just its bare button - `set_cell` spawns a frame the
/// first time a cell is actually played (see `TicTacToeState::cell_entities`).
pub fn setup(room: &mut Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Tic Tac Toe" {
        return;
    }

    send_map_definitions(world);

    let mut cell_block_positions = [BlockPos { x: 0, y: 0, z: 0 }; 9];
    for (i, relative) in CELL_POSITIONS.iter().enumerate() {
        cell_block_positions[i] = room.get_world_block_pos(relative);
    }

    let mut wall_block_positions = [BlockPos { x: 0, y: 0, z: 0 }; 15];
    for (i, relative) in WALL_POSITIONS.iter().enumerate() {
        wall_block_positions[i] = room.get_world_block_pos(relative);
    }

    let reward_chest_pos = room.get_world_block_pos(&REWARD_CHEST_POS);

    let first = cell_block_positions[0];
    let read_back = world.get_block_at(first.x, first.y, first.z);
    // The real direction, used as-is (no flip) for the wire value, yaw, AND position - matching
    // real vanilla exactly (`EntityHanging`/`EntityItemFrame`, decompiled 1.8.9 source): the
    // client's `NetHandlerPlayClient` decodes Object Data via `EnumFacing.getHorizontal(data)`
    // (the same South=0/West=1/North=2/East=3 index `object_data()` already sends) into this
    // exact direction, and `EntityItemFrame`'s constructor immediately calls
    // `updateFacingWithBoundingBox(direction)`, which sets BOTH `rotationYaw` (overriding
    // whatever yaw we send, so `hanging_yaw`/timing never actually mattered) AND recomputes
    // posX/Y/Z itself from `hangingPosition + 0.5 - direction.frontOffset * 0.46875` - the exact
    // formula below. There is no client-side quirk to compensate for; the earlier `.opposite()`
    // flip and the whole position-bisection detour were chasing a different, already-fixed bug
    // (the yaw-write-ordering bug) under confounded test conditions (random cell each restart).
    let facing = match read_back {
        Blocks::StoneButton { direction, .. } => direction.direction(),
        _ => CELL_FACING.rotate(room.rotation),
    };

    const HANGING_DEPTH_OFFSET: f64 = 0.46875;
    let (ox, _, oz) = facing.get_offset();
    let mut cell_positions = [DVec3::new(0.0, 0.0, 0.0); 9];
    for (i, &world_pos) in cell_block_positions.iter().enumerate() {
        cell_positions[i] = DVec3::new(
            world_pos.x as f64 + 0.5 - ox as f64 * HANGING_DEPTH_OFFSET,
            world_pos.y as f64 + 0.5,
            world_pos.z as f64 + 0.5 - oz as f64 * HANGING_DEPTH_OFFSET,
        );
    }

    let state = Rc::new(RefCell::new(TicTacToeState {
        room_index,
        cell_entities: [None; 9],
        cell_positions,
        cell_block_positions,
        wall_block_positions,
        reward_chest_pos,
        facing,
        cell_states: [CellState::Blank; 9],
        game_over: false,
        pending_bot_move: None,
    }));

    for (index, &block_pos) in cell_block_positions.iter().enumerate() {
        world.interactable_blocks.insert(block_pos, BlockInteractAction::TicTacToeButton { state: state.clone(), index });
    }

    let opening_move = CORNERS[rand::rng().random_range(0..CORNERS.len())];
    set_cell(&state, world, opening_move, CellState::X);

    room.tic_tac_toe_state = Some(state);
}

/// Swaps `index`'s displayed cell to `new_state`.
///
/// - `Blank` -> `X`/`O` with no frame yet: destroys that cell's button (clears it to Air, unregisters
///   it from `world.interactable_blocks`) and spawns a new Item Frame in its place holding the
///   target design's filled map - the button is gone the instant a cell is actually played.
/// - already has a frame: reconstructs its `ItemFrame` variant with the new held item and
///   broadcasts via `send_metadata_update`, the same "rebuild the variant, then broadcast"
///   pattern `creeper_beams.rs` uses for its own dynamic Creeper/Guardian metadata.
/// - back to `Blank`: despawns the frame (no reset flow exists to actually reach this today, but
///   kept correct regardless - the button isn't restored, matching this puzzle having no undo).
pub fn set_cell(state: &Rc<RefCell<TicTacToeState>>, world: &mut World, index: usize, new_state: CellState) {
    let (existing_entity, pos, block_pos, facing) = {
        let mut state = state.borrow_mut();
        state.cell_states[index] = new_state;
        (state.cell_entities[index], state.cell_positions[index], state.cell_block_positions[index], state.facing)
    };

    if new_state == CellState::Blank {
        if let Some(entity_id) = existing_entity {
            world.despawn_entity(entity_id);
            state.borrow_mut().cell_entities[index] = None;
        }
        return;
    }

    let item = ItemStack {
        item: MAP_ITEM_ID,
        stack_size: 1,
        metadata: new_state.map_id() as i16,
        tag_compound: None,
    };

    match existing_entity {
        Some(entity_id) => {
            if let Some((entity, _)) = world.entities.get_mut(&entity_id) {
                entity.metadata.variant = EntityVariant::ItemFrame { facing, held_item: Some(item), rotation: 0 };
            }
            world.send_metadata_update(entity_id);
        }
        None => {
            world.interactable_blocks.remove(&block_pos);
            world.set_block_at(Blocks::Air, block_pos.x, block_pos.y, block_pos.z);

            let metadata = EntityMetadata::new(EntityVariant::ItemFrame {
                facing,
                held_item: Some(item),
                rotation: 0,
            });
            // Real vanilla hanging entities (`EntityHanging.setDirection`, decompiled source)
            // explicitly set `this.yaw = direction.b() * 90` - a direction-dependent yaw sent in
            // the `SpawnObject` packet itself (`entity.rs::write_spawn_packet` sends `self.yaw`
            // verbatim), not just Object Data. Must go through `spawn_entity_with_rotation` (not
            // `spawn_entity` + a post-hoc `entity.yaw = ..`) - `spawn_entity` already writes and
            // queues the `SpawnObject` packet with `Entity::new`'s default yaw (0) before
            // returning, so mutating `entity.yaw` afterward has no effect on what was actually
            // sent - every frame was spawned with the same wrong (South's) yaw regardless of its
            // real facing, which is exactly the symmetric-looking-wrong bug this fixes.
            if let Ok(entity_id) = world.spawn_entity_with_rotation(pos, hanging_yaw(facing), 0.0, metadata, NoEntityImpl) {
                state.borrow_mut().cell_entities[index] = Some(entity_id);
            }
        }
    }
}

/// Plays the move-accepted sound pair - a stone button click plus a note pling, fired
/// simultaneously - both originating from the pressed button's own world position, not the
/// player. Broadcast directly to every connected player (not scheduled through
/// `world.scheduled_fixed_sounds`, since there's no delay to model - both go out on the exact
/// same tick as the move itself) so each client pans/fades it by their own distance to the
/// button, exactly like any other positional world sound.
fn play_button_click_sound(world: &mut World, button_pos: BlockPos) {
    let (x, y, z) = (button_pos.x as f64 + 0.5, button_pos.y as f64 + 0.5, button_pos.z as f64 + 0.5);

    // `block.stone_button.click_on` is the 1.9+ name; this project's 1.8.9 protocol unifies it
    // (like every other button/lever click) under the legacy `random.click` event - same
    // `Sounds::RandomClick` already used elsewhere for buttons/levers. The packet's pitch byte
    // is `clamp(pitch * 63, 0, 255) as u8`, which *truncates* rather than rounds:
    // `0.6 * 63 = 37.8` would truncate straight through to byte 37 (pitch `37/63 ≈ 0.587`), but
    // the nearest real byte to `0.6` is 38 (`38/63 ≈ 0.603`). Biasing to the middle of byte 38's
    // rounding bucket makes it truncate to exactly 38 regardless of float noise.
    const CLICK_PITCH_BYTE: u8 = 38;
    let click_pitch = (CLICK_PITCH_BYTE as f32 + 0.5) / 63.0;

    // `block.note_block.pling` -> legacy `note.pling` (`Sounds::NotePling`). The captured pitch
    // `4.048` is well above the 0.5-2.0 range some higher-level sound APIs clamp to, but this
    // project's own packet field only clamps the *encoded byte* to 0..255, not the semantic
    // pitch value - `4.048 * 63 = 255.024`, which the packet's `clamp(.., 0.0, 255.0)` already
    // pins to exactly byte 255 (`0xFF`) before the cast, matching the captured raw byte with no
    // extra biasing needed (unlike the click sound above, there's no truncation ambiguity here).
    let pling_pitch: f32 = 4.048;

    for player in world.players.values_mut() {
        player.write_packet(&SoundEffect {
            sound: Sounds::RandomClick.id(),
            pos_x: x,
            pos_y: y,
            pos_z: z,
            volume: 0.3,
            pitch: click_pitch,
        });
        player.write_packet(&SoundEffect {
            sound: Sounds::NotePling.id(),
            pos_x: x,
            pos_y: y,
            pos_z: z,
            volume: 8.0,
            pitch: pling_pitch,
        });
    }
}

/// `direction.b() * 90` from real vanilla's `EntityHanging` - the same South=0/West=1/North=2/
/// East=3 "type 2" encoding `EntityVariant::ItemFrame::object_data` already uses for the Object
/// Data field, here producing the matching yaw a hanging entity's spawn packet should carry.
fn hanging_yaw(facing: Direction) -> f32 {
    let steps = match facing {
        Direction::South => 0,
        Direction::West => 1,
        Direction::North => 2,
        Direction::East => 3,
        Direction::Up | Direction::Down => 0,
    };
    steps as f32 * 90.0
}

/// Animates the 5x3 Iron Block wall behind the board falling away - the exact same technique a
/// Wither door opens with (`door::DoorEntityImpl`: a falling-block object entity riding an
/// invisible Bat, ticking downward for `FALL_TICKS` ticks before despawning), reused as-is
/// rather than duplicated since it's already generic over the block being dropped. Reads back
/// each position's real current block first (same "trust what's actually placed" approach
/// `setup` already uses for the frames' facing) so this doesn't hardcode an assumed material,
/// swaps it to a temporary Barrier immediately (so the falling copy doesn't double up with a
/// still-solid block), then schedules the real switch to Air once the animation finishes -
/// matching `Door::open_door`'s own barrier-then-air sequencing exactly.
fn drop_wall(world: &mut World, positions: &[BlockPos; 15]) {
    const FALL_TICKS: u32 = 20;

    for &pos in positions {
        let block = world.get_block_at(pos.x, pos.y, pos.z);
        world.set_block_at(Blocks::Barrier, pos.x, pos.y, pos.z);

        let _ = world.spawn_entity(
            DVec3::new(pos.x as f64 + 0.5, pos.y as f64 - DOOR_ENTITY_OFFSET, pos.z as f64 + 0.5),
            {
                let mut metadata = EntityMetadata::new(EntityVariant::Bat { hanging: false });
                metadata.is_invisible = true;
                metadata
            },
            DoorEntityImpl::new(block, 5.0, FALL_TICKS),
        );
    }

    let positions = *positions;
    world.server_mut().schedule(FALL_TICKS, move |server| {
        for pos in positions {
            server.world.set_block_at(Blocks::Air, pos.x, pos.y, pos.z);
        }
    });
}

/// Turns the puzzle's real, already-placed-but-inert reward chest (`REWARD_CHEST_POS`) into an
/// actual `DungeonSecret` "blessing" chest - same shape Teleport Maze's own
/// `reveal_reward_chest` uses, minus `puzzle_room_index`: winning Tic Tac Toe already marks
/// `room.puzzle_completed` and broadcasts PUZZLE SOLVED itself, right in `finish_if_over` (the
/// only caller of this function), so this chest is a bonus reward, not the win condition - it
/// must NOT also carry `puzzle_room_index`, or opening it later would fire a second, redundant
/// PUZZLE SOLVED broadcast.
fn reveal_reward_chest(world: &mut World, reward_chest_pos: BlockPos) {
    // Confirmed in-game to face East, unlike the item frames (`facing`, resolved by reading the
    // placed button back) - hardcoded rather than derived, matching `EXISTING_CHEST`'s own
    // hardcoded `Direction::North` in `teleport_maze.rs::reveal_reward_chest`.
    let secret_rc = Rc::new(RefCell::new(DungeonSecret::new(
        SecretType::Chest { direction: Direction::East },
        reward_chest_pos,
        0.0,
    )));
    {
        let mut secret = secret_rc.borrow_mut();
        secret.blessing_texture = Some(REWARD_BLESSING_TEXTURE);
        // A bonus, not one of the room's actual secrets - winning the game (`finish_if_over`)
        // already counts the puzzle as solved, so this chest must not also bump found_secrets.
        secret.counts_as_secret = false;
    }
    let secret_ref = secret_rc.borrow_mut();
    DungeonSecret::spawn_into_world(&secret_rc, secret_ref, world);
}

/// 9 evenly-spaced clicks (one per board cell) timed alongside `drop_wall`'s fall, all
/// originating from the wall's own center block - index `7` of `WALL_POSITIONS`
/// (`y=71,z=16`, the true middle of the 5x3 grid, since the wall itself is this puzzle's
/// door) - captured once here and reused for every scheduled sound, rather than recomputed
/// later, so a player backing away mid-sequence doesn't change where later clicks appear to
/// come from. Only ever called from `finish_if_over`'s win/tie branch, which itself only
/// runs once per room (gated by `TicTacToeState::game_over` - `interact_cell` refuses any
/// further moves once it's set), so this can't double-schedule on a later update.
fn play_puzzle_complete_sound(world: &mut World, wall_positions: &[BlockPos; 15]) {
    let door_center = wall_positions[7];
    let (x, y, z) = (door_center.x as f64 + 0.5, door_center.y as f64 + 0.5, door_center.z as f64 + 0.5);

    // `block.wooden_pressure_plate.click_on` is the 1.9+ name; this project targets 1.8.9,
    // whose protocol has no separate sound-category field and unified every button/lever/
    // pressure-plate click (including this one) under the single legacy event name
    // `random.click` - the same `Sounds::RandomClick` already used for lever/button clicks
    // elsewhere in this file's sibling `block_interact_action.rs`.
    //
    // Pitch: the packet wire format only has one byte of pitch resolution
    // (`clamp(pitch * 63, 0, 255) as u8`, see `clientbound.rs`'s `SoundEffect`). The
    // requested 0.394 encodes to `0.394 * 63 = 24.822`, which *rounds* to byte 25, but the
    // cast truncates rather than rounds - passing 0.394 straight through would truncate
    // down to byte 24 (client-observed pitch `24/63 ≈ 0.381`), farther from the target than
    // byte 25 (`25/63 ≈ 0.397`). `(25.0 + 0.5) / 63.0` sits in the middle of byte 25's
    // rounding bucket, so it truncates to exactly 25 regardless of float noise.
    const PITCH_BYTE: u8 = 25;
    let pitch = (PITCH_BYTE as f32 + 0.5) / 63.0;

    let world_tick = world.tick_count;
    for i in 0..9u64 {
        world.scheduled_fixed_sounds.push(ScheduledFixedSound {
            due_tick: world_tick + i * 5,
            sound: Sounds::RandomClick,
            volume: 0.6,
            pitch,
            pos_x: x,
            pos_y: y,
            pos_z: z,
        });
    }
}

/// The 8 winning lines over `CELL_POSITIONS`' row-major indexing.
const LINES: [[usize; 3]; 8] = [
    [0, 1, 2], [3, 4, 5], [6, 7, 8],
    [0, 3, 6], [1, 4, 7], [2, 5, 8],
    [0, 4, 8], [2, 4, 6],
];

fn winner(board: &[CellState; 9]) -> Option<CellState> {
    for [a, b, c] in LINES {
        if board[a] != CellState::Blank && board[a] == board[b] && board[b] == board[c] {
            return Some(board[a]);
        }
    }
    None
}

fn is_full(board: &[CellState; 9]) -> bool {
    board.iter().all(|&cell| cell != CellState::Blank)
}

/// Exhaustive minimax score for `board` from the mover named by `maximizing` (`true` = bot/`X`'s
/// turn to move) - `+10`/`-10` for an `X`/`O` win, `0` for a tie, no depth shaping (9 cells is
/// cheap enough to search fully every call - see `best_bot_move`).
fn minimax(board: &mut [CellState; 9], maximizing: bool) -> i32 {
    if let Some(win) = winner(board) {
        return if win == CellState::X { 10 } else { -10 };
    }
    if is_full(board) {
        return 0;
    }
    let mover = if maximizing { CellState::X } else { CellState::O };
    let mut best = if maximizing { i32::MIN } else { i32::MAX };
    for i in 0..9 {
        if board[i] != CellState::Blank {
            continue;
        }
        board[i] = mover;
        let score = minimax(board, !maximizing);
        board[i] = CellState::Blank;
        best = if maximizing { best.max(score) } else { best.min(score) };
    }
    best
}

/// The bot's move: perfect play (see `minimax`), picking uniformly at random among every
/// equally-best-scoring cell rather than always the first found - keeps the bot's non-opening
/// moves from being deterministic/memorizable beyond "it never loses."
fn best_bot_move(board: &[CellState; 9]) -> usize {
    let mut board = *board;
    let mut best_score = i32::MIN;
    let mut best_moves = Vec::new();
    for i in 0..9 {
        if board[i] != CellState::Blank {
            continue;
        }
        board[i] = CellState::X;
        let score = minimax(&mut board, false);
        board[i] = CellState::Blank;
        match score.cmp(&best_score) {
            std::cmp::Ordering::Greater => {
                best_score = score;
                best_moves.clear();
                best_moves.push(i);
            }
            std::cmp::Ordering::Equal => best_moves.push(i),
            std::cmp::Ordering::Less => {}
        }
    }
    best_moves[rand::rng().random_range(0..best_moves.len())]
}

/// `BlockInteractAction::TicTacToeButton`'s handler - a player clicked button `index`. Places
/// their `O`; if the game isn't already decided by that move, computes the bot's own `X` response
/// immediately (`best_bot_move` - perfect play, cheap, no reason to delay the actual thinking) but
/// only *schedules* placing it `BOT_MOVE_DELAY_TICKS` from now (see `tick`) rather than placing it
/// this same tick - per explicit request, the bot should visibly take its time. Ignored if the
/// game is already over, a bot move is already pending (per explicit request - the player can't
/// get a second move in during the bot's own delay), or `index` is already occupied - covers both
/// a player clicking an already-played cell (its button no longer exists by then, so this only
/// matters for the brief window before `set_cell` clears it) and a click landing after the game
/// just ended.
pub fn interact_cell(player: &mut Player, index: usize, state: &Rc<RefCell<TicTacToeState>>) {
    let button_pos = {
        let data = state.borrow();
        if data.game_over || data.pending_bot_move.is_some() || data.cell_states[index] != CellState::Blank {
            return;
        }
        data.cell_block_positions[index]
    };

    let username = player.profile.username.clone();
    let world = player.world_mut();
    play_button_click_sound(world, button_pos);
    set_cell(state, world, index, CellState::O);

    if !finish_if_over(world, &username, state) {
        let bot_move = best_bot_move(&state.borrow().cell_states);
        let fire_tick = world.tick_count + BOT_MOVE_DELAY_TICKS;
        state.borrow_mut().pending_bot_move = Some(PendingBotMove { index: bot_move, fire_tick, username });
    }
}

/// Places the bot's already-computed pending move (see `interact_cell`) the instant its own
/// delay actually elapses - a no-op for every room but the one with an active `tic_tac_toe_state`
/// that currently has a move waiting. Called every tick from `Room::tick`, same pattern as every
/// other puzzle's own per-tick hook (Teleport Maze/Ice Fill/Boulder/Blaze/Water Board).
pub fn tick(room: &Room, world: &mut World) {
    let Some(state_rc) = room.tic_tac_toe_state.clone() else { return };

    let pending = state_rc.borrow().pending_bot_move.clone();
    let Some(pending) = pending else { return };
    if world.tick_count < pending.fire_tick {
        return;
    }

    state_rc.borrow_mut().pending_bot_move = None;
    set_cell(&state_rc, world, pending.index, CellState::X);
    finish_if_over(world, &pending.username, &state_rc);
}

/// Checks `state` for a winner/tie after a move; if the game just ended, marks `game_over`,
/// broadcasts a PUZZLE SOLVED (win or tie - matching the real puzzle, where only a loss fails
/// it) or PUZZLE FAIL message (same bold `§a§l`/`§c§l` prefix + `room.puzzle_completed` pattern
/// every other puzzle's resolve function uses, e.g. `three_weirdos.rs::interact_chest`), and
/// records the
/// puzzle failed with the dungeon's stats on a loss. Returns whether the game ended.
///
/// Takes `world`/`username` rather than a live `&mut Player` since the game can now end from
/// either call site in `interact_cell` (a live `Player` right there) OR from `tick`'s delayed bot
/// move (only ever has `&mut World` - see `PendingBotMove::username`'s own doc comment for why
/// the credited name is captured ahead of time instead of looked up again here).
fn finish_if_over(world: &mut World, username: &str, state: &Rc<RefCell<TicTacToeState>>) -> bool {
    let (result, room_index) = {
        let data = state.borrow();
        let result = winner(&data.cell_states).map(Ok).or_else(|| is_full(&data.cell_states).then_some(Err(())));
        (result, data.room_index)
    };

    let Some(result) = result else { return false };
    state.borrow_mut().game_over = true;

    match result {
        // The bot (`X`) can never actually win against a player who never misses a block, but a
        // tie (`Err`) is the realistic best case - both count as solved, matching the real
        // puzzle where only an outright loss fails it.
        Err(()) | Ok(CellState::O) => {
            let message = format!("§a§lPUZZLE SOLVED! §a{username} §esolved the Tic Tac Toe puzzle!");
            for other in world.players.values_mut() {
                other.send_message(&message);
            }

            let wall_positions = state.borrow().wall_block_positions;
            drop_wall(world, &wall_positions);
            play_puzzle_complete_sound(world, &wall_positions);

            let reward_chest_pos = state.borrow().reward_chest_pos;
            reveal_reward_chest(world, reward_chest_pos);

            // Clear the board's Item Frames (and the maps they're holding) along with the wall -
            // the game is over, nothing left to display. `cell_entities` only has entries for
            // cells actually played (see its own doc comment), so this is a no-op for any cell
            // still bare.
            let cell_entities = state.borrow().cell_entities;
            for entity_id in cell_entities.into_iter().flatten() {
                world.despawn_entity(entity_id);
            }
            state.borrow_mut().cell_entities = [None; 9];
        }
        Ok(_) => {
            let message = format!("§c§lPUZZLE FAIL! §a{username} §elost the Tic Tac Toe puzzle!");
            for other in world.players.values_mut() {
                other.send_message(&message);
            }
            world.server_mut().dungeon.record_puzzle_failed();
        }
    }

    // `Ok(_)` above is the only losing case (`Err(())` is a tie, `Ok(CellState::O)` a win - both
    // handled in the solved arm) - matches it again here rather than threading a bool through,
    // since `result` is still in scope and this stays next to the branch it describes.
    let failed = matches!(result, Ok(CellState::X));
    if let Some(room) = world.server_mut().dungeon.rooms.get_mut(room_index) {
        room.puzzle_completed = true;
        room.puzzle_failed = failed;
    }
    world.server_mut().dungeon.update_map_for_room(room_index);
    true
}

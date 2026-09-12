//! Water Board puzzle - real name confirmed identical ("Water Board" in both this project's
//! captured room data and Odin's own `Puzzle` enum, see `main.rs`'s `puzzle_odin_display_name`).
//!
//! Mechanic (per explicit, direct in-game confirmation - NOT reverse-engineered from a solver
//! mod alone, see the doc comments below for exactly which facts came from where): a master
//! WATER lever retracts a lapis piston holding back real water already sitting above it,
//! releasing it into a wall-mounted maze. 6 other levers (Coal/Gold/Quartz/Diamond/Emerald/Clay)
//! each push/pull every block of their own color scattered through that maze, opening or closing
//! junctions the flowing water passes through. On room entry, 3 of the 5 gates (a color-coded
//! row of small piston doors) are randomly chosen to be closed; reaching a gate with water
//! toggles it. All 5 open -> the reward chest becomes real and openable.
//!
//! This project has no captured data for the maze's actual internal junction/pipe layout and
//! nothing maps which lever-controlled block actually gates which path to which gate (no solver
//! mod needs that - it only needs to tell a player *when* to click, not simulate the maze
//! itself), so there's no way to simulate the puzzle's real *routing logic*. Instead, this treats
//! the real captured answer key (which lever, how many seconds after the water starts, per
//! pattern + which-3-gates-are-closed combination - see `WATER_SOLUTIONS_JSON`) as ground truth:
//! a lever click within `CLICK_TOLERANCE_TICKS` of one of its still-unconsumed expected times
//! counts as correct, and once every expected click across every lever has been consumed, all 3
//! originally closed gates open at once and the chest is revealed. This gets the real difficulty/
//! skill (the correct real sequence and real timing is required) and the real end state, without
//! fabricating a fake per-click-to-specific-gate mapping that doesn't exist in any known source.
//!
//! Separately, real water DOES actually flow once the WATER lever retracts the lapis dam (see
//! `set_water_flow`/`tick`) - driven entirely by [`crate::server::world::fluid::WaterSim`], a
//! reusable, room-scoped reimplementation of real vanilla 1.8.9 water physics
//! (`net.minecraft.block.BlockDynamicLiquid`), not a simplified stand-in. This module owns none
//! of that physics itself: it only ever adds/removes the dam's own source (`set_water_flow`),
//! ticks the sim once per world tick (`tick`), and pokes `WaterSim::notify_neighbors` whenever
//! ITS OWN gate/piston geometry changes (`solve`) - real vanilla's own "a block change wakes up
//! adjacent liquid" behavior, applied to the one kind of geometry change this puzzle actually
//! knows about. See `WaterSim`'s own doc comment for the exact decay/falling/spread rules.
//! Independent of the answer-key win-condition logic above - this is purely a real, believable
//! *visual*, since the maze's routing logic isn't simulated and the water doesn't "know" which
//! lever clicks are correct, it just flows through whatever real open space actually exists.
//!
//! Real captured coordinates (room-local, canonical/pre-rotation) - cross-verified directly
//! in-game by the user, not merely inferred from a client mod:
//! - 5 gates (each a 6-block piston door; position given is its confirmed bottom-center-front
//!   block): Purple (15,56,19), Orange (15,56,18), Blue (15,56,17), Green (15,56,16),
//!   Red (15,56,15). A real wool block there = closed; air = open.
//! - 7 levers (confirmed real `Lever` blocks, already part of the room's own static data - these
//!   only need registering, not placing): Coal (20,61,10), Gold (20,61,15), Quartz (20,61,20),
//!   Diamond (10,61,20), Emerald (10,61,15), Clay (10,61,10), Water (15,60,5).
//! - Lapis dam (15,82,26) - real water confirmed sitting directly above it. Pulling the WATER
//!   lever retracts this block (set to air), letting the real water above it start flowing down
//!   into the room (see `set_water_flow`); pulling it again restores the lapis block and drains
//!   every cell the flow simulation has placed so far.
//! - Water Board stays exactly ONE `room_data_storage` entry (`src/room_data/rooms/
//!   65,water_board,-60,-60.json`) - per explicit correction, the real maze pattern is purely
//!   INTERNAL variety, not 4-5 separate simultaneously-eligible "puzzle rooms" (that would make
//!   real dungeon generation pick Water Board as a puzzle TYPE more often than every other single-
//!   entry puzzle - an earlier version of this integration made exactly that mistake, see this
//!   puzzle's own memory/commit history). 4 real, independently captured full rooms the user
//!   WorldEdited + exported directly (embedded via `PATTERN_0/1/3_JSON`; pattern 2's own capture
//!   turned out to be this same physical room, see `MATERIAL_BLOCKS_ORIGINAL`'s own doc comment)
//!   give this one room 4 genuinely distinct real maze layouts - `pick_pattern` swaps in one
//!   uniformly at random once, at real dungeon-generation time (`Room::load_into_world`, matching
//!   how Boulder/Teleport Maze/Ice Fill already pick their own internal variety there). The real
//!   answer key/switch-panel data (`WATER_SOLUTIONS_JSON`/`switch_panel_raw`) genuinely covers 4
//!   patterns, but only the ORIGINAL room's own pairing to pattern "0" was ever confirmed (see
//!   `answer_key_pattern_for_room`) - the switch panel itself still only ever uses the original
//!   room's own "Board 1" data on every pattern (a known, documented simplification, purely
//!   cosmetic - doesn't affect the puzzle's actual win condition).
//! - Reward chest (15,56,22), confirmed real, facing the room's own canonical north (rotated to
//!   match the room's actual placement, same as every other puzzle's reward chest in this
//!   project) - confirmed openable only once every gate is open.
//! - Each gate is also confirmed to be surrounded by 5 real piston blocks sharing its own z:
//!   2 on the left at x=18 (y=57 and y=56) facing West, 2 on the right at x=12 (y=57 and y=56)
//!   facing East, and 1 underneath at (15,54) facing Up. These physically push inward to
//!   assemble the gate's colored wool "door" when closed - except its topmost-middle block,
//!   which sits out of every piston's reach, so (per explicit confirmation) Hypixel's own admins
//!   apparently couldn't rig a redstone circuit to place it and just spawn it in directly
//!   instead - that's exactly the single wool block this puzzle already placed/removed before
//!   this real piston geometry was confirmed, so no change was needed there.

use crate::dungeon::room::room::Room;
use crate::dungeon::room::secrets::{DungeonSecret, SecretType};
use crate::net::protocol::play::clientbound::SoundEffect;
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_parameter::LeverOrientation;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::player::player::Player;
use crate::server::utils::direction::Direction;
use crate::server::utils::sounds::Sounds;
use crate::server::world::fluid::WaterSim;
use crate::server::world::World;
use crate::utils::seeded_rng::seeded_rng;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Odin's real captured answer key - see the module doc comment for what it does and doesn't
/// tell us. Shape: `[optimized: "true"|"false"][pattern: "0".."3"][extended-slots combo, e.g.
/// "013"][lever solution key, e.g. "gold_block"] -> [click times in seconds since water started]`.
const WATER_SOLUTIONS_JSON: &str = include_str!("../../room_data/misc/puzzles/Waterboard/waterSolutions.json");

type SolutionTable = HashMap<String, HashMap<String, HashMap<String, HashMap<String, Vec<f64>>>>>;

static WATER_SOLUTIONS: Lazy<SolutionTable> = Lazy::new(|| {
    serde_json::from_str(WATER_SOLUTIONS_JSON).expect("waterSolutions.json should always parse - fetched verbatim from Odin's own repo")
});

/// Never opted into "optimized" (a presumably faster/alternate click sequence Odin's data also
/// carries) - the plain default sequence is the one this project has actually verified anything
/// about, so it's the one used here.
const USE_OPTIMIZED: &str = "false";

/// How far off (in ticks) an actual lever click is still allowed to land from one of its
/// expected times and count as correct. Not a captured real value (no known source states real
/// Hypixel's own tolerance) - generous enough to allow for manual reaction-time imprecision
/// while still requiring genuinely following the real timing, matching this project's general
/// preference for a reasonable judgment call over fabricating false precision.
const CLICK_TOLERANCE_TICKS: i64 = 20;

/// How long after room entry the 3 randomly-chosen gates actually push closed (see `setup`) - per
/// explicit correction, real play shows a brief pause before the real piston push happens, not an
/// instant "already sealed the moment the room loads" state. Not a captured real value (no known
/// source states the real exact delay) - just long enough to read as a deliberate reaction rather
/// than instant, matching this project's general preference for a reasonable judgment call over
/// fabricating false precision (see `CLICK_TOLERANCE_TICKS`/`SOLVE_WATER_EFFECT_TICKS` for the
/// same reasoning applied elsewhere in this module).
const GATE_CLOSE_DELAY_TICKS: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LeverKind {
    Coal,
    Gold,
    Quartz,
    Diamond,
    Emerald,
    Clay,
    Water,
}

impl LeverKind {
    const ALL: [LeverKind; 7] = [
        LeverKind::Coal, LeverKind::Gold, LeverKind::Quartz,
        LeverKind::Diamond, LeverKind::Emerald, LeverKind::Clay,
        LeverKind::Water,
    ];

    fn local_pos(self) -> BlockPos {
        match self {
            LeverKind::Coal => BlockPos::new(20, 61, 10),
            LeverKind::Gold => BlockPos::new(20, 61, 15),
            LeverKind::Quartz => BlockPos::new(20, 61, 20),
            LeverKind::Diamond => BlockPos::new(10, 61, 20),
            LeverKind::Emerald => BlockPos::new(10, 61, 15),
            LeverKind::Clay => BlockPos::new(10, 61, 10),
            LeverKind::Water => BlockPos::new(15, 60, 5),
        }
    }

    /// The key `waterSolutions.json` uses for this lever - matches Odin's own
    /// `WaterSolver.kt` block-name mapping exactly (Clay's real block is Hardened Clay/
    /// Terracotta, hence "hardened_clay" rather than "clay_block").
    fn solution_key(self) -> &'static str {
        match self {
            LeverKind::Coal => "coal_block",
            LeverKind::Gold => "gold_block",
            LeverKind::Quartz => "quartz_block",
            LeverKind::Diamond => "diamond_block",
            LeverKind::Emerald => "emerald_block",
            LeverKind::Clay => "hardened_clay",
            LeverKind::Water => "water",
        }
    }

    /// Confirmed real facing of this lever's own physical `Lever` block - Coal/Gold/Quartz sit on
    /// the room's east wall facing West, Diamond/Emerald/Clay sit on the west wall facing East.
    /// Water's own facing (`UpZ`, a floor lever) is handled separately in `set_water_flow`, not
    /// here, since it's driven by a different code path.
    fn orientation(self) -> LeverOrientation {
        match self {
            LeverKind::Coal | LeverKind::Gold | LeverKind::Quartz => LeverOrientation::West,
            LeverKind::Diamond | LeverKind::Emerald | LeverKind::Clay => LeverOrientation::East,
            LeverKind::Water => LeverOrientation::UpZ,
        }
    }

    /// The real block this lever's own material-group puzzle pieces are made of (see
    /// `material_blocks_for_room`) - `None` for `Water`, which owns no material blocks (its own dam is a
    /// `LapisLazuliBlock`, handled separately by `set_water_flow`). Quartz's real captured meta
    /// was always 0 (plain quartz block, not chiseled/pillar).
    fn material_block(self) -> Option<Blocks> {
        match self {
            LeverKind::Coal => Some(Blocks::CoalBlock),
            LeverKind::Gold => Some(Blocks::GoldBlock),
            LeverKind::Quartz => Some(Blocks::QuartzBlock { variant: 0 }),
            LeverKind::Diamond => Some(Blocks::DiamondBlock),
            LeverKind::Emerald => Some(Blocks::EmeraldBlock),
            LeverKind::Clay => Some(Blocks::HardenedClay),
            LeverKind::Water => None,
        }
    }
}

/// One real, piston-driven material-group puzzle block scattered through the maze wall (room-
/// local/pre-rotation `(x, y)`; `initially_extended` is this exact block's own real captured
/// state - `true` if the real room data shows it already pushed out into the channel, `false` if
/// retracted into the wall). Every single one of these 26 real blocks - and every coordinate,
/// state, and the shared piston geometry below - was decoded directly from this project's own
/// already-included real captured room data (`65,water_board,-60,-60.json`'s `block_data`), NOT
/// guessed: scanning that data for the exact real sticky-piston signature (a block whose (x,y,z)
/// has a `PistonHead{North, sticky}` immediately behind it at z+1 and a `StickyPiston{North,
/// extended}` base at z+2 when extended, or plain air at z with the block itself sitting at z+1
/// and a retracted `StickyPiston{North}` base at z+2 when retracted) turned up exactly 6 distinct
/// material groups (Coal/Gold/Quartz/Diamond/Emerald/plain `HardenedClay` - NOT the colored
/// `StainedHardenedClay` the maze's own cyan wall material uses, which is a different block
/// entirely and correctly never matched this signature) plus, unprompted, the already-
/// independently-confirmed lapis dam itself (same signature, same base offset/direction) - see
/// `lapis_piston_local_pos`, which this cross-checks exactly. This is also exactly why a
/// coordinate search alone (matching on material id) isn't good enough: it also finds each
/// lever's own small purely-decorative material cube mounted right next to it at y=61, which
/// this exact piston signature correctly does NOT match (no piston backs them), keeping this
/// table free of anything that isn't a real movable puzzle piece.
struct MaterialBlockSpec {
    lever: LeverKind,
    x: i32,
    y: i32,
    initially_extended: bool,
}

/// This block's real retracted (into the wall) z - one step in front of `MATERIAL_BLOCK_BASE_Z`,
/// room-local/pre-rotation, real captured, same as every other piston's own `near_pos` in this
/// module (see `gate_pistons`/`set_gate_pistons`).
const MATERIAL_BLOCK_RETRACTED_Z: i32 = 27;
/// This block's real extended (pushed out into the flow channel) z - `far_pos`, one step further
/// than `MATERIAL_BLOCK_RETRACTED_Z` in the piston's own facing direction (`North`, i.e. -z).
const MATERIAL_BLOCK_EXTENDED_Z: i32 = 26;
/// Every one of these 26 real pistons' own fixed base z, confirmed real - same base-to-near-to-
/// far spacing (1 block per step) as the lapis dam's own piston (`lapis_piston_local_pos`) and
/// the 5 gates' own piston frames (`gate_pistons`), just always facing `North` here rather than
/// each gate's own left/right/up split.
const MATERIAL_BLOCK_BASE_Z: i32 = 28;

const fn material_block_spec(lever: LeverKind, x: i32, y: i32, initially_extended: bool) -> MaterialBlockSpec {
    MaterialBlockSpec { lever, x, y, initially_extended }
}

/// Every real material-group puzzle block in the ORIGINAL captured room
/// (`65,water_board,-60,-60.json`, `RoomData::id` `"-60,-60"`) - see `MaterialBlockSpec`'s own doc
/// comment for how this table was derived. This project used to have only this one real physical
/// Water Board room capture; a multi-pattern version built by overlaying these same 26 pieces onto
/// OTHER patterns' foreign positions was tried and reverted (pieces kept landing in spots this
/// room's own real capture never built a matching recess for - see this puzzle's own memory/commit
/// history for the full debugging trail). Superseded by 4 more real, independently captured full
/// rooms the user WorldEdited + exported directly (`MATERIAL_BLOCKS_PATTERN_0..3` below,
/// `src/room_data/rooms/65,water_board_patternN,-60,-60.json`) - this table is kept only because
/// the original room file is also kept as a 5th real, independently-selectable variant (see
/// `material_blocks_for_room`), not because anything is still guessed onto it.
const MATERIAL_BLOCKS_ORIGINAL: &[MaterialBlockSpec] = &[
    material_block_spec(LeverKind::Coal, 15, 63, true),
    material_block_spec(LeverKind::Coal, 16, 68, true),
    material_block_spec(LeverKind::Coal, 20, 71, true),
    material_block_spec(LeverKind::Coal, 15, 76, true),

    material_block_spec(LeverKind::Gold, 17, 63, true),
    material_block_spec(LeverKind::Gold, 7, 68, false),
    material_block_spec(LeverKind::Gold, 16, 77, true),

    material_block_spec(LeverKind::Diamond, 10, 62, false),
    material_block_spec(LeverKind::Diamond, 22, 66, true),
    material_block_spec(LeverKind::Diamond, 11, 68, false),
    material_block_spec(LeverKind::Diamond, 17, 70, false),
    material_block_spec(LeverKind::Diamond, 9, 72, false),
    material_block_spec(LeverKind::Diamond, 15, 73, false),

    material_block_spec(LeverKind::Emerald, 6, 63, false),
    material_block_spec(LeverKind::Emerald, 11, 63, false),
    material_block_spec(LeverKind::Emerald, 20, 66, true),
    material_block_spec(LeverKind::Emerald, 14, 68, false),

    material_block_spec(LeverKind::Quartz, 20, 62, false),
    material_block_spec(LeverKind::Quartz, 7, 64, false),
    material_block_spec(LeverKind::Quartz, 9, 68, false),
    material_block_spec(LeverKind::Quartz, 22, 71, true),
    material_block_spec(LeverKind::Quartz, 16, 74, false),

    material_block_spec(LeverKind::Clay, 10, 67, true),
    material_block_spec(LeverKind::Clay, 14, 77, false),
    material_block_spec(LeverKind::Clay, 16, 62, true),
    material_block_spec(LeverKind::Clay, 21, 70, true),
];

/// Every real material-group puzzle block in 3 of the 4 real, independently captured full Water
/// Board rooms the user WorldEdited + exported and sent directly (`src/room_data/misc/puzzles/
/// Waterboard/65,water_board_patternN,-60,-60.json`, embedded via `PATTERN_N_JSON` below) - decoded
/// with the exact same real sticky-piston-signature scan as `MATERIAL_BLOCKS_ORIGINAL` (see
/// `pattern_decode::decode_material_block_patterns`, a `#[test] #[ignore]` tool kept in this file
/// for regenerating these tables if the real captures are ever redone, mirroring `boulder.rs`'s own
/// `convert_boulder_patterns` convention). Pattern 2's own real capture turned out to be the exact
/// same physical layout as `MATERIAL_BLOCKS_ORIGINAL` (set-identical, 26-for-26, per the user's own
/// confirmation this room's static data literally IS pattern 2) - only its own file's 4 stray
/// decorative `Leaf` blocks differ, so it reuses `MATERIAL_BLOCKS_ORIGINAL` directly rather than
/// duplicating an identical table (see `material_blocks_for_pattern`). These tables are used ONLY
/// for runtime lever-toggle bookkeeping (`set_material_group`'s `initially_extended` inversion) -
/// the actual INITIAL placement is whatever `pick_pattern` already swapped `block_data` to at real
/// dungeon-generation time, no placement/overlay call needed at `setup` time at all.
const MATERIAL_BLOCKS_PATTERN_0: &[MaterialBlockSpec] = &[
    material_block_spec(LeverKind::Quartz, 6, 61, true),
    material_block_spec(LeverKind::Gold, 7, 75, false),
    material_block_spec(LeverKind::Clay, 8, 71, false),
    material_block_spec(LeverKind::Coal, 9, 74, false),
    material_block_spec(LeverKind::Clay, 10, 61, false),
    material_block_spec(LeverKind::Gold, 10, 71, true),
    material_block_spec(LeverKind::Coal, 11, 63, false),
    material_block_spec(LeverKind::Gold, 12, 64, true),
    material_block_spec(LeverKind::Diamond, 13, 77, true),
    material_block_spec(LeverKind::Quartz, 14, 78, true),
    material_block_spec(LeverKind::Diamond, 15, 65, true),
    material_block_spec(LeverKind::Clay, 15, 75, false),
    material_block_spec(LeverKind::Emerald, 16, 76, true),
    material_block_spec(LeverKind::Emerald, 16, 78, false),
    material_block_spec(LeverKind::Diamond, 17, 63, false),
    material_block_spec(LeverKind::Gold, 17, 65, true),
    material_block_spec(LeverKind::Quartz, 17, 75, false),
    material_block_spec(LeverKind::Coal, 17, 77, false),
    material_block_spec(LeverKind::Clay, 18, 64, false),
    material_block_spec(LeverKind::Clay, 18, 67, false),
    material_block_spec(LeverKind::Gold, 18, 76, true),
    material_block_spec(LeverKind::Coal, 21, 64, false),
    material_block_spec(LeverKind::Emerald, 23, 64, false),
];

const MATERIAL_BLOCKS_PATTERN_1: &[MaterialBlockSpec] = &[
    material_block_spec(LeverKind::Emerald, 6, 62, true),
    material_block_spec(LeverKind::Gold, 6, 66, true),
    material_block_spec(LeverKind::Emerald, 8, 66, false),
    material_block_spec(LeverKind::Diamond, 9, 69, true),
    material_block_spec(LeverKind::Emerald, 9, 78, true),
    material_block_spec(LeverKind::Diamond, 10, 62, false),
    material_block_spec(LeverKind::Gold, 11, 77, false),
    material_block_spec(LeverKind::Emerald, 14, 72, false),
    material_block_spec(LeverKind::Diamond, 14, 78, false),
    material_block_spec(LeverKind::Emerald, 16, 64, false),
    material_block_spec(LeverKind::Quartz, 16, 78, true),
    material_block_spec(LeverKind::Quartz, 18, 77, false),
    material_block_spec(LeverKind::Diamond, 19, 66, false),
    material_block_spec(LeverKind::Gold, 20, 77, false),
    material_block_spec(LeverKind::Gold, 21, 70, false),
    material_block_spec(LeverKind::Quartz, 22, 66, true),
    material_block_spec(LeverKind::Diamond, 22, 73, false),
    material_block_spec(LeverKind::Emerald, 22, 77, false),
];

const MATERIAL_BLOCKS_PATTERN_3: &[MaterialBlockSpec] = &[
    material_block_spec(LeverKind::Emerald, 6, 62, true),
    material_block_spec(LeverKind::Coal, 6, 68, false),
    material_block_spec(LeverKind::Quartz, 9, 63, false),
    material_block_spec(LeverKind::Diamond, 9, 69, false),
    material_block_spec(LeverKind::Coal, 10, 62, false),
    material_block_spec(LeverKind::Gold, 10, 74, true),
    material_block_spec(LeverKind::Emerald, 10, 76, true),
    material_block_spec(LeverKind::Quartz, 11, 69, false),
    material_block_spec(LeverKind::Clay, 11, 73, false),
    material_block_spec(LeverKind::Diamond, 12, 74, false),
    material_block_spec(LeverKind::Emerald, 13, 71, true),
    material_block_spec(LeverKind::Gold, 14, 64, true),
    material_block_spec(LeverKind::Clay, 14, 66, false),
    material_block_spec(LeverKind::Quartz, 14, 78, false),
    material_block_spec(LeverKind::Clay, 15, 63, true),
    material_block_spec(LeverKind::Coal, 15, 71, false),
    material_block_spec(LeverKind::Emerald, 16, 62, true),
    material_block_spec(LeverKind::Diamond, 16, 64, true),
    material_block_spec(LeverKind::Gold, 16, 66, true),
    material_block_spec(LeverKind::Emerald, 16, 74, true),
    material_block_spec(LeverKind::Gold, 16, 78, true),
    material_block_spec(LeverKind::Gold, 17, 71, true),
    material_block_spec(LeverKind::Emerald, 19, 71, true),
    material_block_spec(LeverKind::Quartz, 20, 70, true),
    material_block_spec(LeverKind::Coal, 20, 77, true),
    material_block_spec(LeverKind::Diamond, 21, 64, false),
    material_block_spec(LeverKind::Clay, 21, 71, false),
    material_block_spec(LeverKind::Clay, 21, 74, false),
    material_block_spec(LeverKind::Emerald, 22, 63, false),
    material_block_spec(LeverKind::Diamond, 22, 73, false),
    material_block_spec(LeverKind::Gold, 22, 75, true),
    material_block_spec(LeverKind::Coal, 23, 64, false),
    material_block_spec(LeverKind::Emerald, 23, 74, true),
];

/// This room instance's own real material-block table, keyed by `RoomData::id` - `pick_pattern`
/// (called once from `Room::load_into_world`, generation time) tags every Water Board room's own
/// `id` as `"water_board_pattern_0".."_3"` regardless of which pattern was actually picked, so this
/// lookup never has to know about the original file's own raw id. Pattern 2 reuses
/// `MATERIAL_BLOCKS_ORIGINAL` directly (see that const's own doc comment for why - the real capture
/// turned out to already be the same physical layout).
fn material_blocks_for_room(room: &Room) -> &'static [MaterialBlockSpec] {
    match room.room_data.id.as_str() {
        "water_board_pattern_0" => MATERIAL_BLOCKS_PATTERN_0,
        "water_board_pattern_1" => MATERIAL_BLOCKS_PATTERN_1,
        "water_board_pattern_3" => MATERIAL_BLOCKS_PATTERN_3,
        _ => MATERIAL_BLOCKS_ORIGINAL, // "water_board_pattern_2" and any unexpected id
    }
}

/// This room instance's own index into `WATER_SOLUTIONS_JSON`'s real per-pattern answer key
/// (`"0".."3"`). NOT arbitrary - this must match whatever Odin's own solver (`WaterSolver.kt::scan`,
/// fetched directly from `github.com/odtheking/Odin`) will independently detect live in-game by
/// reading 3 real decorative marker blocks (Terracotta at (14,77,27) -> 0, EmeraldBlock at
/// (16,78,27) -> 1, DiamondBlock at (14,78,27) -> 2, QuartzBlock at (14,78,27) -> 3, first match
/// wins) - Odin has no idea which of this project's own real captured files is active, it only
/// ever reads the live world. An earlier version of this function assigned indices by decode/piece-
/// count order instead of by what Odin actually detects, which happened to match for patterns 0/1
/// (id -> answer key "0"/"1") but NOT for patterns 1/3 (the "water_board_pattern_1" file's own real
/// marker is DiamondBlock - Odin's own pattern 2 - not "3"; the "water_board_pattern_3" file's own
/// real marker is QuartzBlock - Odin's own pattern 3 - not "2") - confirmed by directly reproducing
/// Odin's exact detection logic against each file's own real captured block_data
/// (`pattern_decode::check_odin_pattern_detection_matches`, a `#[test] #[ignore]` kept in this file
/// for re-verifying this any time the captured files change) rather than guessing. That mismatch
/// silently made this server enforce a DIFFERENT lever-timing answer key than what Odin displayed
/// to the player for those 2 of 4 real patterns - "Odin's solver sometimes doesn't work" was this,
/// not a bug in Odin or in the capture data itself.
fn answer_key_pattern_for_room(room: &Room) -> &'static str {
    match room.room_data.id.as_str() {
        "water_board_pattern_0" => "1",
        "water_board_pattern_1" => "2",
        "water_board_pattern_3" => "3",
        _ => "0", // "water_board_pattern_2" (confirmed - see MATERIAL_BLOCKS_ORIGINAL) and any unexpected id
    }
}

/// The 3 other real full room captures the user WorldEdited + exported directly (pattern 2's own
/// capture is the original room itself, see `MATERIAL_BLOCKS_ORIGINAL`'s doc comment) - embedded
/// here (not left in `src/room_data/rooms/`) specifically so they're NOT independently visible to
/// `get_random_data_with_type`'s own real dungeon-generation room picker. Per explicit correction:
/// Water Board must stay exactly ONE `room_data_storage` entry - the real maze pattern is purely
/// INTERNAL variety, picked by `pick_pattern` below, not 4 separate simultaneously-eligible
/// "puzzle rooms" (that would make real generation pick Water Board as a puzzle TYPE more often
/// than every other single-entry puzzle, not just vary its own internal look - the mistake an
/// earlier version of this integration made, see this puzzle's own memory/commit history).
const PATTERN_0_JSON: &str = include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern0,-60,-60.json");
const PATTERN_1_JSON: &str = include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern1,-60,-60.json");
const PATTERN_3_JSON: &str = include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern3,-60,-60.json");

/// Picks this Water Board room instance's own real maze pattern (0-3, uniform) and swaps
/// `room_data.block_data` to match, tagging `room_data.id` as `"water_board_pattern_N"` either way
/// so every other Water Board lookup in this module (`material_blocks_for_room`,
/// `answer_key_pattern_for_room`) can tell which pattern this specific room instance is without
/// re-rolling anything itself. Called ONCE from `Room::load_into_world`, at real dungeon-generation
/// time - matches how Boulder/Teleport Maze/Ice Fill already pick their own internal variety there,
/// before the room's static blocks actually get placed into the world, so the maze is already
/// correct the instant a player can see it (through a window, on the map, etc.), not silently
/// re-rolled later at room-entry time the way the *gates* deliberately are (see the module doc
/// comment for why that timing distinction matters there but not here).
///
/// Pattern 2 needs no `block_data` swap at all - it's the exact same real physical room this
/// `RoomData` already IS (per the user's own explicit confirmation, matching the near-total
/// decode-level match found independently beforehand) - only its own `id` gets tagged, same as
/// every other pattern, so `material_blocks_for_room`/`answer_key_pattern_for_room` don't need a
/// special case for "no swap happened".
pub fn pick_pattern(room_data: &mut crate::dungeon::room::room_data::RoomData) {
    apply_pattern(room_data, rand::Rng::random_range(&mut seeded_rng(), 0..4));
}

/// The actual swap-and-tag logic `pick_pattern` uses, factored out so `practice::find_room_data`'s
/// own `water_board_N` special case can deterministically pick a SPECIFIC pattern for testing
/// (`pattern` here is caller-supplied, not rolled) instead of duplicating this logic.
pub fn apply_pattern(room_data: &mut crate::dungeon::room::room_data::RoomData, pattern: u8) {
    let pattern = pattern % 4;
    if pattern != 2 {
        let raw = match pattern {
            0 => PATTERN_0_JSON,
            1 => PATTERN_1_JSON,
            _ => PATTERN_3_JSON,
        };
        room_data.block_data = crate::dungeon::room::room_data::RoomData::from_raw_json(raw).block_data;
    }
    room_data.id = format!("water_board_pattern_{pattern}");
}

/// This spec's 3 world-space positions (base, retracted/near, extended/far) - used both to place
/// blocks (`apply_material_block`) and to tell `WaterSim::notify_neighbors` which cells a move
/// just touched.
fn material_block_positions(room: &Room, spec: &MaterialBlockSpec) -> [BlockPos; 3] {
    [
        room.get_world_block_pos(&BlockPos::new(spec.x, spec.y, MATERIAL_BLOCK_BASE_Z)),
        room.get_world_block_pos(&BlockPos::new(spec.x, spec.y, MATERIAL_BLOCK_RETRACTED_Z)),
        room.get_world_block_pos(&BlockPos::new(spec.x, spec.y, MATERIAL_BLOCK_EXTENDED_Z)),
    ]
}

/// Places `spec`'s own real sticky piston + material block to match `extended`, atomically (base,
/// then whichever of near/far actually holds the block, with the other explicitly cleared to air)
/// so a block can never appear at both positions, neither, or duplicate - same real sticky-piston
/// shape as `set_gate_pistons`/`set_dam_piston` (base fixed, block always resting in exactly one
/// of `near_pos`/`far_pos`). Always derives its blocks fresh from `extended`, never from whatever
/// happens to already be there, so calling this repeatedly can never drift.
fn apply_material_block(room: &Room, world: &mut World, spec: &MaterialBlockSpec, extended: bool) {
    let material = spec.lever.material_block().expect("no material-block table entry ever targets LeverKind::Water");
    let dir = Direction::North.rotate(room.rotation);
    let [base_pos, near_pos, far_pos] = material_block_positions(room, spec);

    world.set_block_at(Blocks::StickyPiston { direction: dir, extended }, base_pos.x, base_pos.y, base_pos.z);
    if extended {
        world.set_block_at(Blocks::PistonHead { direction: dir, sticky: true }, near_pos.x, near_pos.y, near_pos.z);
        world.set_block_at(material, far_pos.x, far_pos.y, far_pos.z);
    } else {
        world.set_block_at(material, near_pos.x, near_pos.y, near_pos.z);
        world.set_block_at(Blocks::Air, far_pos.x, far_pos.y, far_pos.z);
    }
}

/// Moves every one of `lever`'s own real material-group blocks to match `powered`: each block's
/// target state is simply its OWN fixed real captured `initially_extended` state, inverted if
/// `powered` - "the lever inverts the starting state of the entire material group", per the
/// module doc comment, derived fresh from that fixed initial state on every single call (never
/// from wherever a block currently happens to be), so repeated toggles can never drift, duplicate,
/// or leave anything stuck between two positions. No-op for `LeverKind::Water` (see
/// `LeverKind::material_block`). Wakes `WaterSim` for every cell any block's move actually
/// touches (`WaterSim::notify_neighbors`) - `WaterSim` itself has no idea what a "lever" or
/// "material" is, same "Water Board only pokes it" pattern the gates already use; the sim's own
/// normal decay/spread rules are entirely responsible for how water actually reacts to the new
/// geometry - this never places, removes, or reroutes water directly.
fn set_material_group(room: &Room, world: &mut World, water_sim: &mut WaterSim, now: u64, lever: LeverKind, powered: bool) {
    for spec in material_blocks_for_room(room).iter().filter(|spec| spec.lever == lever) {
        let desired_extended = spec.initially_extended != powered;
        apply_material_block(room, world, spec, desired_extended);
        for pos in material_block_positions(room, spec) {
            water_sim.notify_neighbors(world, pos, now);
        }
    }
}

/// Raw `[x, y, z]` board-relative coordinate from the user's own `WaterBoardPractice/util/
/// redstone_blocks.js` (`redstoneBlockLocations`), copied verbatim - NOT yet in this project's
/// own room-local space, see `switch_panel_local_pos` for the bridge.
type RawPos = (i32, i32, i32);

/// Every position in the switch-panel wall that shows `RedstoneBlock` while `lever` is pulled
/// (`Stone` otherwise) - the real, per-lever "Board 1" data from `redstoneBlockLocations`, copied
/// verbatim from the source file so it stays directly cross-checkable against it. Applied
/// unconditionally to every one of the 5 real room variants (see `material_blocks_for_room`'s own
/// doc comment) - a deliberate, documented simplification: this project has no confirmed per-
/// pattern switch-panel data (unlike the material blocks, which now come from real per-pattern
/// captures), and the panel is purely cosmetic, not part of the actual win condition.
fn switch_panel_raw(lever: LeverKind) -> &'static [RawPos] {
    match lever {
        LeverKind::Water => &[(0, 63, -14)],
        LeverKind::Coal => &[(-6, 45, -14), (4, 44, -14), (6, 55, -14), (-2, 58, -14)],
        LeverKind::Gold => &[(3, 45, -14), (-2, 46, -14), (-3, 57, -14), (5, 52, -14), (8, 56, -14)],
        LeverKind::Quartz => &[(9, 42, -14), (-2, 56, -14), (1, 59, -14)],
        LeverKind::Diamond => &[(-2, 44, -14), (0, 46, -14), (2, 58, -14)],
        LeverKind::Emerald => &[(-1, 59, -14), (-1, 57, -14), (-8, 45, -14)],
        LeverKind::Clay => &[(5, 42, -14), (-3, 45, -14), (-3, 48, -14), (0, 56, -14), (7, 52, -14)],
    }
}

/// Bridges a raw `switch_panel_raw` coordinate (from `redstoneBlockLocations`) into this project's
/// own room-local space, "relative to the lapis dam". The X/Y offsets match this puzzle's earlier
/// validated bridge formula, but the Z constant here is different (+17, not +11) - confirmed by
/// cross-referencing directly against this project's own real captured room data
/// (`65,water_board,-60,-60.json`, which genuinely has `redstone_block`s baked in at z=29):
/// `redstoneBlockLocations`'s universal `Water` entry `[0, 63, -14]` (identical on every board)
/// lands EXACTLY on a real captured `redstone_block` at (15, 82, 29) with this constant, an exact
/// 3-axis match - the earlier +11 constant was validated against a *different* capture
/// (`pattern1..4.json`, an unrelated blank reset-template scan) and doesn't apply to this dataset,
/// same "different captures use different, non-interchangeable reference frames" caveat this
/// puzzle's research already ran into once before with `leverLocations`/`gateLocations`.
fn switch_panel_local_pos((rx, ry, rz): RawPos) -> BlockPos {
    let dam = lapis_dam_local_pos();
    BlockPos::new(dam.x + rx, dam.y + (ry - 63), dam.z + (rz + 17))
}

/// Sets every one of `lever`'s switch-panel positions to `RedstoneBlock` (powered) or plain
/// `Stone` (unpowered), matching real vanilla's own redstone-block/stone indicator convention.
fn set_switch_panel(room: &Room, world: &mut World, lever: LeverKind, powered: bool) {
    let block = if powered { Blocks::RedstoneBlock } else { Blocks::Stone { variant: 0 } };
    for &raw in switch_panel_raw(lever) {
        let pos = room.get_world_block_pos(&switch_panel_local_pos(raw));
        world.set_block_at(block, pos.x, pos.y, pos.z);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateColor {
    Purple,
    Orange,
    Blue,
    Green,
    Red,
}

impl GateColor {
    const ALL: [GateColor; 5] = [GateColor::Purple, GateColor::Orange, GateColor::Blue, GateColor::Green, GateColor::Red];

    fn local_pos(self) -> BlockPos {
        match self {
            GateColor::Purple => BlockPos::new(15, 56, 19),
            GateColor::Orange => BlockPos::new(15, 56, 18),
            GateColor::Blue => BlockPos::new(15, 56, 17),
            GateColor::Green => BlockPos::new(15, 56, 16),
            GateColor::Red => BlockPos::new(15, 56, 15),
        }
    }

    /// Real confirmed water-detection sensor position - where water actually has to physically
    /// arrive to toggle this gate (distinct from `local_pos`, the gate door itself: this sits
    /// higher up and spread along X, matching the maze's own output end, not the door's row
    /// along Z). Not used for any real routing simulation (no captured plumbing/junction data
    /// exists to route real water through), only for showing a real, correctly-positioned water
    /// arrival effect once the puzzle is solved - see `solve`.
    fn sensor_pos(self) -> BlockPos {
        match self {
            GateColor::Purple => BlockPos::new(24, 58, 26),
            GateColor::Orange => BlockPos::new(20, 58, 26),
            GateColor::Blue => BlockPos::new(15, 58, 26),
            GateColor::Green => BlockPos::new(10, 58, 26),
            GateColor::Red => BlockPos::new(6, 58, 26),
        }
    }

    /// Real captured vanilla wool color metadata. Orange/Green/Red confirmed directly (these 3
    /// happened to be the closed ones in the one real room instance checked); Purple/Blue use
    /// vanilla's standard DyeColor values since both were open (air) in that capture and their
    /// real wool color was never directly observed - safe to correct later if it turns out wrong,
    /// this only affects cosmetic color, never the puzzle logic.
    fn wool_metadata(self) -> u8 {
        match self {
            GateColor::Purple => 10,
            GateColor::Orange => 1,
            GateColor::Blue => 11,
            GateColor::Green => 5,
            GateColor::Red => 14,
        }
    }

    /// This color's digit in the extended-slots combo string `waterSolutions.json` keys by
    /// (e.g. "013") - matches Odin's own `WoolColor` enum's declaration order exactly.
    fn solution_index(self) -> u8 {
        match self {
            GateColor::Purple => 0,
            GateColor::Orange => 1,
            GateColor::Blue => 2,
            GateColor::Green => 3,
            GateColor::Red => 4,
        }
    }
}

/// The 5 real piston blocks physically surrounding this gate (see the module doc comment) -
/// 2 on the left facing West, 2 on the right facing East, 1 underneath facing Up, all sharing
/// this gate's own z. Directions are room-local/pre-rotation, same as every other local
/// coordinate in this module - callers must rotate them the same way `local_pos` positions get
/// rotated via `Room::get_world_block_pos`.
fn gate_pistons(gate: GateColor) -> [(BlockPos, Direction); 5] {
    let z = gate.local_pos().z;
    [
        (BlockPos::new(18, 57, z), Direction::West),
        (BlockPos::new(18, 56, z), Direction::West),
        (BlockPos::new(12, 57, z), Direction::East),
        (BlockPos::new(12, 56, z), Direction::East),
        (BlockPos::new(15, 54, z), Direction::Up),
    ]
}

/// The single "top middle" wool position (room-local/pre-rotation), centered between the two
/// side pistons' top row and out of the bottom piston's reach - the one piece of the door no
/// piston can reach at all, confirmed hand-spawned directly rather than pushed (see the module
/// doc comment).
fn gate_top_middle_pos(gate: GateColor) -> BlockPos {
    BlockPos::new(15, 57, gate.local_pos().z)
}

/// Extends (`closed`) or retracts every one of `gate`'s 5 real surrounding pistons, moving its
/// attached colored wool block along with it - these are real STICKY pistons (per the user's
/// own in-game observation: a retracted piston's wool doesn't just vanish, it stays visible
/// resting against the piston face), so at any given time each piston has its wool sitting in
/// exactly one of two spots: immediately against the base (`near_pos`, retracted/open, resting)
/// or pushed one space further out (`far_pos`, extended/closed, filling the gate) - never both,
/// never neither. The piston's own head render only ever shows at `near_pos`, and only while
/// extended (a sticky piston's retracted head has nothing to show once its block returns home).
fn set_gate_pistons(room: &Room, world: &mut World, gate: GateColor, closed: bool) {
    let color = gate.wool_metadata();
    for (local_pos, local_dir) in gate_pistons(gate) {
        let dir = local_dir.rotate(room.rotation);
        let base_pos = room.get_world_block_pos(&local_pos);
        world.set_block_at(Blocks::StickyPiston { direction: dir, extended: closed }, base_pos.x, base_pos.y, base_pos.z);

        let (dx, dy, dz) = dir.get_offset();
        let near_pos = BlockPos::new(base_pos.x + dx, base_pos.y + dy, base_pos.z + dz);
        let far_pos = BlockPos::new(base_pos.x + dx * 2, base_pos.y + dy * 2, base_pos.z + dz * 2);

        if closed {
            world.set_block_at(Blocks::PistonHead { direction: dir, sticky: true }, near_pos.x, near_pos.y, near_pos.z);
            world.set_block_at(Blocks::Wool { color }, far_pos.x, far_pos.y, far_pos.z);
        } else {
            world.set_block_at(Blocks::Wool { color }, near_pos.x, near_pos.y, near_pos.z);
            world.set_block_at(Blocks::Air, far_pos.x, far_pos.y, far_pos.z);
        }
    }
}

/// Sets `gate`'s own real geometry (top-middle wool + the 5-piston frame) to match `closed`,
/// with no side effects beyond the blocks themselves (no sound, no `WaterboardState`/`WaterSim`
/// bookkeeping) - the shared block-placement half of both `setup`'s initial roll and
/// `set_gate_state`'s later live toggling.
fn apply_gate_state(room: &Room, world: &mut World, gate: GateColor, closed: bool) {
    let top_middle_pos = room.get_world_block_pos(&gate_top_middle_pos(gate));
    let top_middle_block = if closed { Blocks::Wool { color: gate.wool_metadata() } } else { Blocks::Air };
    world.set_block_at(top_middle_block, top_middle_pos.x, top_middle_pos.y, top_middle_pos.z);
    set_gate_pistons(room, world, gate, closed);
}

/// Broadcasts the real piston sound for `gate` moving to `closed` - `PistonIn` (pistons
/// extending inward to build the door) or `PistonOut` (retracting to open it), matching real
/// vanilla's own distinct extend/retract sounds.
fn play_gate_sound(world: &mut World, room: &Room, gate: GateColor, closed: bool) {
    let pos = room.get_world_block_pos(&gate.local_pos());
    let sound = SoundEffect {
        sound: if closed { Sounds::PistonIn.id() } else { Sounds::PistonOut.id() },
        pos_x: pos.x as f64,
        pos_y: pos.y as f64,
        pos_z: pos.z as f64,
        volume: 1.0,
        pitch: 1.0,
    };
    for player in world.players.values_mut() {
        player.write_packet(&sound);
    }
}

/// Every world-space position `set_gate_pistons`/the top-middle wool can possibly change for
/// `gate` (its own `near_pos`/`far_pos` per piston, plus the top-middle position) - used only to
/// tell `WaterSim::notify_neighbors` which cells just changed when a gate opens (see `solve`),
/// not for placing anything itself.
fn gate_block_positions(room: &Room, gate: GateColor) -> Vec<BlockPos> {
    let mut positions = vec![room.get_world_block_pos(&gate_top_middle_pos(gate))];
    for (local_pos, local_dir) in gate_pistons(gate) {
        let dir = local_dir.rotate(room.rotation);
        let base_pos = room.get_world_block_pos(&local_pos);
        let (dx, dy, dz) = dir.get_offset();
        positions.push(BlockPos::new(base_pos.x + dx, base_pos.y + dy, base_pos.z + dz));
        positions.push(BlockPos::new(base_pos.x + dx * 2, base_pos.y + dy * 2, base_pos.z + dz * 2));
    }
    positions
}

/// Moves `gate` to `closed` for real: places its blocks (`apply_gate_state`), records the new
/// state in `WaterboardState::gate_closed`, plays the matching sound, and wakes `WaterSim` for
/// every position that just changed (`WaterSim::notify_neighbors`) - real vanilla's own "a block
/// change wakes up adjacent liquid", the one hook `WaterSim` needs and the only thing this
/// module has to do to keep it in sync with its own gate geometry. Used both by real-time sensor
/// detection (`update_gate_sensors`) and by `solve`'s own end-of-puzzle guarantee.
fn set_gate_state(room: &Room, world: &mut World, state: &mut WaterboardState, gate: GateColor, closed: bool, now: u64) {
    apply_gate_state(room, world, gate, closed);
    state.gate_closed.insert(gate, closed);
    play_gate_sound(world, room, gate, closed);
    for pos in gate_block_positions(room, gate) {
        state.water_sim.notify_neighbors(world, pos, now);
    }
}

/// Checks every one of the 5 gates' real `GateColor::sensor_pos` against the room's actual
/// current water and toggles any gate real water has JUST reached (edge-triggered via
/// `WaterboardState::gate_sensor_wet`, so a gate doesn't flip again every tick while still wet) -
/// the confirmed real mechanic ("water reaching a gate's position toggles it... not a one-way
/// trigger", see the module doc comment), now driven by this room's own real `WaterSim` instead
/// of the answer-key timing check being the only thing gates ever react to. No-op once solved -
/// `solve` itself is the one thing allowed to move gates after that point.
///
/// If a toggle just left every one of the 5 gates genuinely open, that alone completes the
/// puzzle and calls `solve` directly - matching real Hypixel (the chest unlocks once every gate
/// is really open, full stop), rather than leaving the chest locked behind the separate answer-
/// key timing check even after a player has physically gotten every gate open for real. The
/// timing check (`handle_lever`) is left in place as an alternate, earlier trigger for the same
/// `solve` - whichever condition is met first wins, both guarded by the same `state.solved` flag
/// so `solve` never runs twice.
fn update_gate_sensors(room: &Room, world: &mut World, state_rc: &Rc<RefCell<WaterboardState>>, now: u64) {
    let mut any_toggled = false;
    {
        let mut state = state_rc.borrow_mut();
        if state.solved {
            return;
        }
        for &gate in &GateColor::ALL {
            let sensor_pos = room.get_world_block_pos(&gate.sensor_pos());
            let is_wet = matches!(world.get_block_at(sensor_pos.x, sensor_pos.y, sensor_pos.z), Blocks::StillWater { .. } | Blocks::FlowingWater { .. });
            let was_wet = state.gate_sensor_wet.insert(gate, is_wet).unwrap_or(false);
            if is_wet && !was_wet {
                let now_closed = !*state.gate_closed.get(&gate).unwrap_or(&false);
                set_gate_state(room, world, &mut state, gate, now_closed, now);
                any_toggled = true;
            }
        }
    }
    if !any_toggled {
        return;
    }

    let (room_index, all_open) = {
        let state = state_rc.borrow();
        let all_open = GateColor::ALL.iter().all(|gate| !*state.gate_closed.get(gate).unwrap_or(&false));
        (state.room_index, all_open)
    };
    if all_open {
        state_rc.borrow_mut().solved = true;
        solve(world, room_index, state_rc);
    }
}

/// The real captured blessing skull texture - same constant (by value) as every other puzzle's
/// own reward chest in this project, each keeping its own copy rather than importing a shared
/// one (see `boulder.rs`'s doc comment for why).
const REWARD_BLESSING_TEXTURE: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZTkzZTIwNjg2MTc4NzJjNTQyZWNkYTFkMjdkZjRlY2U5MWM2OTk5MDdiZjMyN2M0ZGRiODUzMDk0MTJkMzkzOSJ9fX0=";

#[derive(Debug)]
pub struct WaterboardState {
    room_index: usize,
    /// The 3 gates randomly chosen closed this run - see `setup`.
    closed_gates: Vec<GateColor>,
    /// `waterSolutions.json[USE_OPTIMIZED][answer_key_pattern_for_room(room)][combo]` for this
    /// run's specific `closed_gates` combo - `None` if that combo somehow isn't in the table
    /// (shouldn't happen for a real 3-of-5 combo, but this project prefers a safe no-op over a
    /// panic on unexpected data). See `answer_key_pattern_for_room`'s own doc comment for how this
    /// room's own real maze layout maps to one of the answer key's 4 pattern indices.
    solution: Option<HashMap<String, Vec<f64>>>,
    /// How many of each lever's expected times (in the order `solution` lists them) have been
    /// correctly consumed so far.
    consumed: HashMap<&'static str, usize>,
    water_on: bool,
    water_start_tick: Option<u64>,
    solved: bool,
    /// Whether each of the 6 color levers is currently pulled - drives both its own physical
    /// `Lever` block's visual state and its switch-panel positions (see `set_switch_panel`).
    /// Independent of `consumed`/the timing check below: this is a plain on/off toggle, not tied
    /// to whether a given click actually counted as "correct".
    lever_powered: HashMap<LeverKind, bool>,
    /// This room's own reusable vanilla water simulation (see
    /// `crate::server::world::fluid::WaterSim`'s own doc comment) - the dam is the sim's only
    /// source (added/removed by `set_water_flow`), scoped to this room's own world bounds so it
    /// can never flow outside it.
    water_sim: WaterSim,
    /// Each of the 5 gates' current REAL open/closed state - starts as `closed_gates` (see
    /// above), but unlike that fixed initial list, this changes live as real water actually
    /// reaches each gate's own `GateColor::sensor_pos` (see `update_gate_sensors`) - per the
    /// confirmed real mechanic, reaching a gate's sensor TOGGLES it, open or closed, not a
    /// one-way trigger, so a gate that started open can later close again if water sloshes back
    /// over it, same as any of the other 5 states.
    gate_closed: HashMap<GateColor, bool>,
    /// Whether real water was sitting at each gate's sensor position as of the last check -
    /// `update_gate_sensors` only toggles a gate on the instant water NEWLY arrives (this flips
    /// false -> true), not every tick it happens to still be sitting there.
    gate_sensor_wet: HashMap<GateColor, bool>,
    /// The tick each lever last had a click actually register (debounce bookkeeping) - see
    /// `LEVER_DEBOUNCE_TICKS`.
    last_click_tick: HashMap<LeverKind, u64>,
}

/// Room-local (min, max) corners spanning every real reference point that defines the puzzle's
/// own channel - the dam, the 5 gates' own real water-arrival sensors, and every real material
/// block's base/retracted/extended position for THIS room's own actual pattern
/// (`material_blocks_for_room`) - shared by `valid_water_cells` and `output_row_positions` so both
/// derive from exactly the same real, already-confirmed data (never a fresh hardcoded coordinate).
/// The sensors are real captured geometry's own lowest point (below every material block and the
/// dam), which is exactly why `min.y` alone is already the real output row's own y with no
/// separate lookup needed - see both callers.
fn channel_reference_corners(room: &Room) -> (BlockPos, BlockPos) {
    let dam = lapis_dam_local_pos();
    let mut min = dam;
    let mut max = dam;
    let mut expand = |p: BlockPos| {
        min.x = min.x.min(p.x); max.x = max.x.max(p.x);
        min.y = min.y.min(p.y); max.y = max.y.max(p.y);
        min.z = min.z.min(p.z); max.z = max.z.max(p.z);
    };

    for &gate in &GateColor::ALL {
        expand(gate.sensor_pos());
    }
    for spec in material_blocks_for_room(room) {
        expand(BlockPos::new(spec.x, spec.y, MATERIAL_BLOCK_BASE_Z));
        expand(BlockPos::new(spec.x, spec.y, MATERIAL_BLOCK_RETRACTED_Z));
        expand(BlockPos::new(spec.x, spec.y, MATERIAL_BLOCK_EXTENDED_Z));
    }
    (min, max)
}

/// The exact, exhaustive set of real world-space cells this room's `WaterSim` may ever place,
/// move, or search through water in - not a bounding volume with holes patched into it, a plain
/// enumerated membership set (see `WaterSim::new`'s own doc comment: a destination outside this
/// set is invalid no matter what direction or code path reaches for it). Built entirely from
/// `channel_reference_corners`'s own real, already-confirmed reference points, transformed to
/// world space one cell at a time via `room.get_world_block_pos` - never assuming which raw world
/// axis is "forward" or "sideways", since that depends entirely on `room.rotation` - so this stays
/// correct under every room orientation, not only whichever one this room was captured facing.
///
/// Earlier attempts at this used a padded bounding BOX (`x`/`y`/`z` ranges with a handful of
/// exceptions carved out for specific known holes) and kept missing real leak paths one at a
/// time, because a box can only say "in vs. out" at its own outer edge - it can't exclude real
/// open space sitting well inside that edge unless every single such hole is separately found and
/// patched. Directly decoding this room's own real captured `block_data` (the same technique
/// already used for `MATERIAL_BLOCKS`/the gate piston geometry, not guesswork) found that this
/// puzzle's real vertical channel only ever needs exactly TWO `z` values, never a padded range:
/// - `MATERIAL_BLOCK_EXTENDED_Z` (26) - the actual flow plane the dam, every gate's real sensor,
///   and every material block's own extended position all sit on.
/// - `MATERIAL_BLOCK_RETRACTED_Z` (27) - purely the material blocks' own retracted rest position.
///
/// Every `z` outside that pair turned out to be a real, but entirely unrelated, leak path once a
/// gate/lever opened a way into it - never guessed, each confirmed by decoding the surrounding
/// layers too, not assumed from one data point:
/// - `z = 25` (one step in front of the flow plane, the maze's own visible glass front face) is
///   real solid glass almost everywhere, but has two real gaps with nothing to do with the actual
///   puzzle flow: the broken shared layer below (`min.y + 1`, see below) and a second one right at
///   the dam's own height (`y = 79..=82`). Simply never including `z = 25` at all closes both.
/// - `z >= MATERIAL_BLOCK_BASE_Z` (28) is the pistons' own real mechanical housing - a huge open
///   cavity (confirmed: real Air in 14-19 of 19 sampled columns at almost every `y` from 59 to
///   79) with no relation to the visible puzzle channel at all. Simply never including `z >= 28`
///   closes this off entirely too, with no need to map the cavity's own real shape.
///
/// The one exception carved out of the otherwise-rectangular `x`/`y` extent is `y = min.y + 1`
/// (one real layer above the output row) at `z = MATERIAL_BLOCK_EXTENDED_Z`: real captured data
/// confirms this ONE layer is genuinely open Air across the whole width with none of the real
/// per-column separation every neighboring layer has (the output row itself has real Wool/
/// Cobblestone dividers; the layers immediately above it have real Stone with gaps only at each
/// gate's own `sensor_pos().x`) - only those same 5 real straight-through columns stay valid
/// here, so a puzzle output can still be reached vertically without the layer acting as a shared
/// trough. Every other cell in the rectangle relies on this room's own real solid geometry
/// (confirmed by direct decode, e.g. the `x` edges are real solid Stone at both `z` values across
/// the entire `y` range) to naturally keep water from reaching anywhere it shouldn't - the mask
/// only ever needs to correct for real Air the puzzle was never meant to expose to simulated
/// water, never to reproduce the geometry that's already correctly solid on its own.
///
/// `MATERIAL_BLOCK_RETRACTED_Z` (27) is further restricted to exactly the real `y` range this
/// room's own `material_blocks_for_room` spans - the only reason that `z` ever needs to be valid
/// at all is as those blocks' own retracted rest position, and decoding confirmed it's real solid
/// Stone everywhere else in the channel's normal `y` range EXCEPT three real, unrelated gaps right
/// at the dam's own height (`y = 80..=82`, alongside the `z = 25` gap already excluded above) -
/// never guessed, confirmed the same way as every other exclusion in this function.
fn valid_water_cells(room: &Room) -> HashSet<BlockPos> {
    let (min, max) = channel_reference_corners(room);
    let sensor_xs: Vec<i32> = GateColor::ALL.iter().map(|g| g.sensor_pos().x).collect();
    let shaft_y = min.y + 1;
    let material_blocks = material_blocks_for_room(room);
    let material_min_y = material_blocks.iter().map(|s| s.y).min().unwrap_or(min.y);
    let material_max_y = material_blocks.iter().map(|s| s.y).max().unwrap_or(max.y);

    let mut cells = HashSet::new();
    for x in min.x..=max.x {
        for y in min.y..=(max.y + 1) {
            for z in [MATERIAL_BLOCK_EXTENDED_Z, MATERIAL_BLOCK_RETRACTED_Z] {
                if y == shaft_y && z == MATERIAL_BLOCK_EXTENDED_Z && !sensor_xs.contains(&x) {
                    continue;
                }
                if z == MATERIAL_BLOCK_RETRACTED_Z && !(material_min_y..=material_max_y).contains(&y) {
                    continue;
                }
                cells.insert(room.get_world_block_pos(&BlockPos::new(x, y, z)));
            }
        }
    }
    cells
}

/// Every real world-space cell along the puzzle's own single output row - the same `y`
/// (`channel_reference_corners`'s own `min.y`, the sensors' real shared height) and `z`
/// (`MATERIAL_BLOCK_EXTENDED_Z`, the real flow plane), spanning the channel's own real `x` range.
/// Registered with `WaterSim` as terminal cells (`WaterSim::add_terminal`) so each real output
/// along this row behaves as its own independent sink - fed only from directly above, never
/// spreading sideways into a neighboring output - instead of the whole row silently acting as one
/// shared horizontal trough just because the space between outputs happens to be open air.
fn output_row_positions(room: &Room) -> Vec<BlockPos> {
    let (min, max) = channel_reference_corners(room);
    (min.x..=max.x)
        .map(|x| room.get_world_block_pos(&BlockPos::new(x, min.y, MATERIAL_BLOCK_EXTENDED_Z)))
        .collect()
}

/// Sets up the Water Board puzzle for `room` if it actually is one - detects which real captured
/// switch-panel layout applies, randomly closes 3 of the 5 gates (placing each gate's real
/// surrounding piston frame extended, plus its directly-spawned wool door block), and registers
/// the 7 already-placed levers as interactable. No-op for every other room.
///
/// Per explicit confirmation, the 5 gates are real 6-block piston doors - Hypixel's own admins
/// apparently couldn't rig a redstone circuit to reliably place the doors' middle-top block, so
/// they're spawned in directly instead. This project doesn't build that full 6-block structure
/// (deferred - see the module doc comment), but it does match the real *timing*: the 3 gates are
/// only pushed shut once a player actually steps into the room (with a real piston sound), not
/// silently at dungeon generation before anyone's there to see or hear it. Called once per room
/// instance from `Dungeon::tick`'s room-entry hook, guarded by `Room::waterboard_spawned` so
/// walking out and back in doesn't re-roll which gates are closed or re-register the levers.
///
/// Per explicit correction, the 3 chosen gates don't slam shut in the same instant a player
/// crosses the threshold either - real play shows a brief pause before they actually push closed,
/// not an instant "already sealed the moment you can see the room" state. `GATE_CLOSE_DELAY_TICKS`
/// below defers only that real closing push (and its sound); every other part of this function -
/// the levers becoming interactable, `WaterboardState`/`solution` bookkeeping, the material blocks'
/// own resting position - stays immediate, since none of that is the thing confirmed delayed.
pub fn setup(room: &mut Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Water Board" {
        return;
    }

    let mut rng = seeded_rng();

    // Water starts off, so the dam piston starts extended/closed - a real lapis block sealing the
    // gap, same as it'd be found on a fresh room.
    set_dam_piston(room, world, true);

    // Every color lever (and Water's own indicator) starts unpulled - the switch panel should
    // show all Stone (off), not whatever the room's own captured static decoration looks like.
    let mut lever_powered = HashMap::new();
    for &lever in &LeverKind::ALL {
        lever_powered.insert(lever, false);
        set_switch_panel(room, world, lever, false);
    }

    let mut all_gates = GateColor::ALL;
    all_gates.shuffle(&mut rng);
    let closed_gates: Vec<GateColor> = all_gates[..3].to_vec();

    // The other 2 gates' own resting-open geometry is real, present from the instant the room is
    // even visible (nothing ever animates them) - only the 3 chosen-closed gates get the delayed
    // real push below.
    for &gate in &GateColor::ALL {
        if !closed_gates.contains(&gate) {
            apply_gate_state(room, world, gate, false);
        }
    }
    {
        let gates_to_close = closed_gates.clone();
        world.server_mut().schedule(GATE_CLOSE_DELAY_TICKS, move |server| {
            for &gate in &gates_to_close {
                let Some(room) = server.dungeon.rooms.get(room_index) else { continue };
                apply_gate_state(room, &mut server.world, gate, true);
                play_gate_sound(&mut server.world, room, gate, true);
            }
        });
    }
    let gate_closed: HashMap<GateColor, bool> = GateColor::ALL.iter().map(|&g| (g, closed_gates.contains(&g))).collect();
    let gate_sensor_wet: HashMap<GateColor, bool> = GateColor::ALL.iter().map(|&g| (g, false)).collect();

    // No material-block placement needed here (unlike an earlier, reverted design) - this room's
    // own real captured `block_data` already shows every piece at its own real starting position,
    // since it's a genuine independent capture now, not reused wall geometry with foreign pieces
    // stitched in (see `material_blocks_for_room`'s own doc comment). No lever has been pulled yet,
    // so nothing needs inverting either (see `set_material_group`'s own doc comment).

    let combo = combo_key(&closed_gates);
    let solution = WATER_SOLUTIONS.get(USE_OPTIMIZED)
        .and_then(|by_pattern| by_pattern.get(answer_key_pattern_for_room(room)))
        .and_then(|by_combo| by_combo.get(&combo))
        .cloned();

    // `valid_water_cells` is the exact, exhaustive set of cells this room's water may ever occupy
    // (see its own doc comment) - every real output along the single output row is additionally
    // marked terminal, so each one fills only from directly above and never spreads sideways into
    // a neighboring output.
    let mut water_sim = WaterSim::new(valid_water_cells(room));
    for pos in output_row_positions(room) {
        water_sim.add_terminal(pos);
    }

    let state_rc = Rc::new(RefCell::new(WaterboardState {
        room_index,
        closed_gates,
        solution,
        consumed: HashMap::new(),
        water_on: false,
        water_start_tick: None,
        solved: false,
        lever_powered,
        water_sim,
        gate_closed,
        gate_sensor_wet,
        last_click_tick: HashMap::new(),
    }));

    for &lever in &LeverKind::ALL {
        let pos = room.get_world_block_pos(&lever.local_pos());
        world.set_block_at(
            Blocks::Lever { orientation: lever.orientation().rotate(room.rotation), powered: false },
            pos.x, pos.y, pos.z,
        );
        world.interactable_blocks.insert(pos, BlockInteractAction::WaterboardLever {
            state: state_rc.clone(),
            lever,
        });
    }

    room.waterboard_state = Some(state_rc);
}

/// Builds the extended-slots combo key (e.g. "013") `waterSolutions.json` is keyed by - digits
/// in ascending order, matching Odin's own `WoolColor.entries` iteration order.
fn combo_key(closed_gates: &[GateColor]) -> String {
    let mut indices: Vec<u8> = closed_gates.iter().map(|g| g.solution_index()).collect();
    indices.sort_unstable();
    indices.iter().map(|i| i.to_string()).collect()
}

/// How many ticks must pass after a lever's last registered click before another click on that
/// SAME lever counts as a new, independent one. This is the actual real-world root cause behind
/// "Odin's solver sometimes doesn't work": Odin's own `WaterSolver.kt` (`waterInteract`) has a
/// comment explaining that a single real right-click sometimes reaches the server as TWO separate
/// interact packets - if the first one's interaction result isn't a hard success, the client
/// automatically retries with the off-hand, and the server "treats [that] as a regular click,
/// causing a double lever flick i.e. the gate doesn't open" (Odin has to filter this client-side
/// by only counting `interactionResult == SUCCESS`, which this project has no equivalent of at
/// the packet layer). Without a matching guard here, that phantom second click silently advances
/// `consumed` an extra step for one lever, permanently misaligning every one of ITS remaining
/// real clicks against the wrong queued expected time for the rest of that run - intermittent by
/// nature, since it only bites on the runs where a duplicate packet actually happens to occur.
/// 4 ticks (200ms) is far shorter than any deliberate second pull a player could mean for a
/// timing puzzle whose real expected clicks are spaced in whole seconds, but comfortably covers a
/// same-instant duplicate packet.
const LEVER_DEBOUNCE_TICKS: u64 = 4;

/// A lever was clicked. `Water` toggles the flow (and resets timing/progress on every toggle -
/// per explicit confirmation, real play involves flicking it off and on to re-examine the board,
/// which restarts the timing clock without undoing already-opened gates, since gates only ever
/// react to water newly arriving, never to it leaving). Every other lever only does anything
/// while the water is on: a click within `CLICK_TOLERANCE_TICKS` of its next unconsumed expected
/// time (per `WATER_SOLUTIONS`) counts as correct; once every lever's entire expected sequence is
/// consumed, all 3 originally-closed gates open and the chest is revealed.
pub fn handle_lever(player: &mut Player, state_rc: &Rc<RefCell<WaterboardState>>, lever: LeverKind) {
    {
        let now = player.world_mut().tick_count;
        let mut state = state_rc.borrow_mut();
        if let Some(&last) = state.last_click_tick.get(&lever) {
            if now.saturating_sub(last) < LEVER_DEBOUNCE_TICKS {
                return;
            }
        }
        state.last_click_tick.insert(lever, now);
    }

    if lever == LeverKind::Water {
        let (room_index, now_on) = {
            let mut state = state_rc.borrow_mut();
            state.water_on = !state.water_on;
            if state.water_on {
                state.water_start_tick = Some(player.world_mut().tick_count);
                state.consumed.clear();
            } else {
                state.water_start_tick = None;
            }
            (state.room_index, state.water_on)
        };
        set_water_flow(player, room_index, state_rc, now_on);
        return;
    }

    let (room_index, now_powered) = {
        let mut state = state_rc.borrow_mut();
        let now_powered = !*state.lever_powered.get(&lever).unwrap_or(&false);
        state.lever_powered.insert(lever, now_powered);
        (state.room_index, now_powered)
    };
    set_lever_visuals(player, room_index, lever, now_powered);

    // Moves this lever's own real material-group puzzle blocks (see `set_material_group`) -
    // unconditional, regardless of whether the water is currently on or already solved, exactly
    // like a real physical piston lever doesn't care about either. The levers only ever change
    // the board's own geometry; `WaterSim`'s normal decay/spread rules (woken via
    // `WaterSim::notify_neighbors` inside `set_material_group`) are entirely responsible for
    // however the water actually reacts - nothing here places, removes, or reroutes water itself.
    if let Some(room) = player.server_mut().dungeon.rooms.get(room_index) {
        let now = player.world_mut().tick_count;
        let world = player.world_mut();
        let mut state = state_rc.borrow_mut();
        set_material_group(room, world, &mut state.water_sim, now, lever, now_powered);
    }

    let (room_index, just_solved) = {
        let mut state = state_rc.borrow_mut();
        if state.solved || !state.water_on {
            return;
        }
        let Some(start_tick) = state.water_start_tick else { return };
        let Some(solution) = state.solution.clone() else { return };
        let Some(expected_times) = solution.get(lever.solution_key()) else { return };

        let now = player.world_mut().tick_count;
        let elapsed_ticks = now.saturating_sub(start_tick) as i64;

        let next_index = *state.consumed.get(lever.solution_key()).unwrap_or(&0);
        let Some(&expected_seconds) = expected_times.get(next_index) else { return };
        let expected_ticks = (expected_seconds * 20.0).round() as i64;

        if (elapsed_ticks - expected_ticks).abs() > CLICK_TOLERANCE_TICKS {
            return;
        }

        state.consumed.insert(lever.solution_key(), next_index + 1);

        let fully_solved = solution.iter()
            .filter(|(key, _)| **key != "water")
            .all(|(key, times)| *state.consumed.get(key.as_str()).unwrap_or(&0) >= times.len());

        if fully_solved {
            state.solved = true;
        }

        (state.room_index, fully_solved)
    };

    if just_solved {
        solve(player.world_mut(), room_index, state_rc);
    }
}

/// How long (ticks) the capstone water-arrival effect (see `solve`) stays visible at each solved
/// gate's sensor position before reverting to air. Not a captured real value (no source states
/// how long real Hypixel's own water lingers there) - just long enough to read as "water arrived"
/// before clearing itself.
const SOLVE_WATER_EFFECT_TICKS: u32 = 60;

/// Guarantees every one of the 5 gates is actually open (`set_gate_state`, which also plays the
/// real sound and wakes `WaterSim`) and reveals the reward chest as a real, openable blessing
/// chest - per explicit confirmation, the chest is only openable once every gate is open, and
/// opening it (not merely reaching this state) is what actually completes the puzzle, same
/// "chest's own opening is the real win condition" pattern Boulder/Teleport Maze/Ice Path use.
///
/// Checks all 5 gates, not just the 3 originally closed: real per-gate sensor detection
/// (`update_gate_sensors`) can toggle ANY gate live as water actually reaches it, including one
/// that started open closing again if water sloshes back over its sensor mid-puzzle - so by the
/// time the answer-key timing check actually completes, an originally-open gate could well be
/// sitting closed for real, and the chest's own "every gate must be open" invariant has to cover
/// that, not just assume the other 2 never moved.
///
/// Also places real water at each of the 3 originally-closed gates' confirmed sensor positions,
/// briefly, purely as a capstone visual for the ones the player actually had to work to open -
/// this project has no captured plumbing/junction data to simulate real water actually routing
/// there per lever click (see the module doc comment), so this doesn't attempt to show water
/// arriving progressively mid-puzzle beyond what real-time sensor detection already does, only a
/// real, correctly-positioned "it arrived" effect once solving is confirmed.
fn solve(world: &mut World, room_index: usize, state_rc: &Rc<RefCell<WaterboardState>>) {
    let Some(rotation) = world.server_mut().dungeon.rooms.get(room_index).map(|room| room.rotation) else { return };
    let closed_gates = state_rc.borrow().closed_gates.clone();

    for &gate in &GateColor::ALL {
        let Some(room) = world.server_mut().dungeon.rooms.get(room_index) else { continue };
        let now = world.tick_count;
        let already_open = !*state_rc.borrow().gate_closed.get(&gate).unwrap_or(&false);
        if already_open {
            continue;
        }
        let mut state = state_rc.borrow_mut();
        set_gate_state(room, world, &mut state, gate, false, now);
    }

    for gate in closed_gates {
        let Some(room) = world.server_mut().dungeon.rooms.get(room_index) else { continue };
        let sensor_pos = room.get_world_block_pos(&gate.sensor_pos());

        world.set_block_at(Blocks::StillWater { level: 0 }, sensor_pos.x, sensor_pos.y, sensor_pos.z);
        // Matches the water this just placed, so `update_gate_sensors` doesn't treat it as a
        // fresh arrival and try to toggle the (already just-forced-open) gate a second time.
        state_rc.borrow_mut().gate_sensor_wet.insert(gate, true);

        world.server_mut().schedule(SOLVE_WATER_EFFECT_TICKS, move |server| {
            server.world.set_block_at(Blocks::Air, sensor_pos.x, sensor_pos.y, sensor_pos.z);
        });
    }

    let Some(chest_pos) = world.server_mut().dungeon.rooms.get(room_index).map(|room| room.get_world_block_pos(&BlockPos::new(15, 56, 22))) else { return };

    let mut secret = DungeonSecret::new(SecretType::Chest { direction: Direction::North.rotate(rotation) }, chest_pos, 0.0);
    secret.blessing_texture = Some(REWARD_BLESSING_TEXTURE);
    secret.has_spawned = true;
    secret.puzzle_room_index = Some(room_index);
    secret.counts_as_secret = false;
    let secret_rc = Rc::new(RefCell::new(secret));
    let secret_mut = secret_rc.borrow_mut();
    DungeonSecret::spawn_into_world(&secret_rc, secret_mut, world);
}

/// The lapis dam's own piston base - per explicit correction, this piston sits BEHIND the lapis
/// block (not underneath it) and pushes/pulls it horizontally in and out. Its exact coordinate
/// was never independently confirmed in-game the way the 5 gates' piston coordinates were - placed
/// 2 blocks further back (+z, toward the same back wall the pattern-detection blocks at z=27 also
/// sit against) from the dam's own confirmed position (15,82,26), facing North so it pushes
/// forward to plug the hole - a reasonable best guess following the same "base + 2×direction
/// offset lands exactly on the already-confirmed real coordinate" pattern the gates fit (see
/// `gate_pistons`), not a verified real position. Correct this if the user ever confirms it.
fn lapis_piston_local_pos() -> BlockPos {
    BlockPos::new(15, 82, 28)
}

/// Extends (`closed`) or retracts the lapis dam's piston, moving its attached lapis block along
/// with it - same real sticky-piston mechanic (and the same code shape) as `set_gate_pistons` uses
/// for the 5 gates: the block rests 1 space out when retracted (open, water flowing) and gets
/// pushed 2 spaces out - onto the dam's own confirmed real position - when extended (closed,
/// blocking the water).
fn set_dam_piston(room: &Room, world: &mut World, closed: bool) {
    let dir = Direction::North.rotate(room.rotation);
    let base_pos = room.get_world_block_pos(&lapis_piston_local_pos());
    world.set_block_at(Blocks::StickyPiston { direction: dir, extended: closed }, base_pos.x, base_pos.y, base_pos.z);

    let (dx, dy, dz) = dir.get_offset();
    let near_pos = BlockPos::new(base_pos.x + dx, base_pos.y + dy, base_pos.z + dz);
    let far_pos = BlockPos::new(base_pos.x + dx * 2, base_pos.y + dy * 2, base_pos.z + dz * 2);

    if closed {
        world.set_block_at(Blocks::PistonHead { direction: dir, sticky: true }, near_pos.x, near_pos.y, near_pos.z);
        world.set_block_at(Blocks::LapisLazuliBlock, far_pos.x, far_pos.y, far_pos.z);
    } else {
        world.set_block_at(Blocks::LapisLazuliBlock, near_pos.x, near_pos.y, near_pos.z);
        world.set_block_at(Blocks::Air, far_pos.x, far_pos.y, far_pos.z);
    }
}

/// The lapis dam's own position (see the module doc comment) - the far/extended landing spot of
/// `lapis_piston_local_pos`'s own piston (confirmed real; see `set_dam_piston`), and the point the
/// water flow simulation seeds itself from once that piston retracts.
fn lapis_dam_local_pos() -> BlockPos {
    BlockPos::new(15, 82, 26)
}

/// Toggles one of the 6 color levers' own physical `Lever` block and its switch-panel positions
/// (see `set_switch_panel`) - called on every single click, regardless of whether that click also
/// happens to land within the timing window checked right after this in `handle_lever`, since a
/// real lever's own on/off state doesn't care about that; it just reflects whatever position it
/// was last left in. Not used for `LeverKind::Water`, whose own physical lever is handled by
/// `set_water_flow` instead, alongside the dam it's tied to.
fn set_lever_visuals(player: &mut Player, room_index: usize, lever: LeverKind, powered: bool) {
    let Some(room) = player.server_mut().dungeon.rooms.get(room_index) else { return };
    let lever_pos = room.get_world_block_pos(&lever.local_pos());
    let orientation = lever.orientation().rotate(room.rotation);
    let world = player.world_mut();

    world.set_block_at(Blocks::Lever { orientation, powered }, lever_pos.x, lever_pos.y, lever_pos.z);
    set_switch_panel(room, world, lever, powered);
}

/// Turns the WATER lever's real effect on/off: flips its own `Lever` block's visual state, and
/// pushes the lapis dam's own piston in or out every single time this is called, unconditionally
/// - per explicit correction, that piston physically moves on every click regardless of anything
/// else going on. Opening it seeds `WaterSim` with a real permanent source at the dam's own now-
/// open position (fed by the real static water confirmed sitting directly above it); closing it
/// removes that source, letting the piston shove the lapis block back into that exact cell
/// (displacing whatever water was there, exactly like a real piston pushing into water does) -
/// `WaterSim` does the rest for free: its own continuous neighbor-recomputation naturally finds
/// the cut cells no longer have a valid supply and dries them out on its own over the next few
/// waves, cascading outward from the cut exactly like real vanilla decay, not some bespoke
/// "puzzle reset" rule.
fn set_water_flow(player: &mut Player, room_index: usize, state_rc: &Rc<RefCell<WaterboardState>>, on: bool) {
    let Some((lever_pos, rotation)) = player.server_mut().dungeon.rooms.get(room_index).map(|room| (
        room.get_world_block_pos(&LeverKind::Water.local_pos()),
        room.rotation,
    )) else { return };

    {
        let world = player.world_mut();
        world.set_block_at(
            Blocks::Lever { orientation: LeverOrientation::UpZ.rotate(rotation), powered: on },
            lever_pos.x, lever_pos.y, lever_pos.z,
        );
    }

    let Some(room) = player.server_mut().dungeon.rooms.get(room_index) else { return };
    let dam_pos = room.get_world_block_pos(&lapis_dam_local_pos());
    let world = player.world_mut();
    set_dam_piston(room, world, !on);
    set_switch_panel(room, world, LeverKind::Water, on);

    let now = world.tick_count;
    let mut state = state_rc.borrow_mut();
    if on {
        state.water_sim.add_source(world, dam_pos, now);
    } else {
        state.water_sim.remove_source(world, dam_pos, now);
    }
}

/// Advances this room's `WaterSim` by one world tick, then checks every gate's real sensor
/// against the water that tick just placed (`update_gate_sensors`) - a no-op for every room but
/// the one with an active `waterboard_state` whose sim still owns a source or has a pending
/// recompute (see `WaterSim::is_active`). All of the actual decay/falling/spread/tick-delay logic
/// lives in `WaterSim` itself; this is purely the per-room wiring `Room::tick` needs.
pub fn tick(room: &Room, world: &mut World) {
    if room.room_data.name != "Water Board" {
        return;
    }
    let Some(state_rc) = room.waterboard_state.clone() else { return };
    let now = world.tick_count;
    {
        let mut state = state_rc.borrow_mut();
        if !state.water_sim.is_active() {
            return;
        }
        state.water_sim.tick(world, now);
    }
    update_gate_sensors(room, world, &state_rc, now);
}

#[cfg(test)]
mod pattern_decode {
    use super::*;
    use crate::dungeon::room::room_data::RoomData;

    fn block_at(data: &RoomData, x: i32, y: i32, z: i32) -> Blocks {
        if x < 0 || x >= data.width || z < 0 || z >= data.length || y < data.bottom {
            return Blocks::Air;
        }
        let layers = data.block_data.len() as i32 / (data.width * data.length);
        if y - data.bottom >= layers {
            return Blocks::Air;
        }
        let index = (x + z * data.width + (y - data.bottom) * data.width * data.length) as usize;
        data.block_data.get(index).cloned().unwrap_or(Blocks::Air)
    }

    fn lever_for_material(block: &Blocks) -> Option<LeverKind> {
        match block {
            Blocks::CoalBlock => Some(LeverKind::Coal),
            Blocks::GoldBlock => Some(LeverKind::Gold),
            Blocks::QuartzBlock { .. } => Some(LeverKind::Quartz),
            Blocks::DiamondBlock => Some(LeverKind::Diamond),
            Blocks::EmeraldBlock => Some(LeverKind::Emerald),
            Blocks::HardenedClay => Some(LeverKind::Clay),
            _ => None,
        }
    }

    /// Regenerates the real per-pattern `MaterialBlockSpec` tables from the 4 real full room
    /// captures the user WorldEdited + exported (`room_data/misc/puzzles/Waterboard/
    /// 65,water_board_patternN,-60,-60.json`) - the same real sticky-piston-signature scan
    /// technique `MATERIAL_BLOCKS`'s own doc comment describes (a block whose (x,y,z) has a
    /// `PistonHead{North,sticky}` immediately behind it at z+1 and a `StickyPiston{North,extended}`
    /// base at z+2 when extended, or the block itself at z+1 with a retracted `StickyPiston{North}`
    /// base at z+2 when retracted), just run against these 4 NEW full captures directly instead of
    /// reusing the old overlay-derived tables. Not run by default - a one-off data-prep step, not a
    /// correctness test. Run with `cargo test decode_material_block_patterns -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn decode_material_block_patterns() {
        let raw_files: [&str; 4] = [
            include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern0,-60,-60.json"),
            include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern1,-60,-60.json"),
            include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern2,-60,-60.json"),
            include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern3,-60,-60.json"),
        ];

        for (p, raw) in raw_files.iter().enumerate() {
            let data = RoomData::from_raw_json(raw);
            println!("const MATERIAL_BLOCKS_PATTERN_{}: &[MaterialBlockSpec] = &[", p);
            let mut count = 0;
            for x in 0..data.width {
                for y in data.bottom..(data.bottom + 40) {
                    let base = block_at(&data, x, y, MATERIAL_BLOCK_BASE_Z);
                    if let Blocks::StickyPiston { direction: Direction::North, extended } = base {
                        if extended {
                            let head = block_at(&data, x, y, MATERIAL_BLOCK_RETRACTED_Z);
                            let material = block_at(&data, x, y, MATERIAL_BLOCK_EXTENDED_Z);
                            if matches!(head, Blocks::PistonHead { direction: Direction::North, sticky: true }) {
                                if let Some(lever) = lever_for_material(&material) {
                                    println!("    material_block_spec(LeverKind::{:?}, {}, {}, true),", lever, x, y);
                                    count += 1;
                                }
                            }
                        } else {
                            let material = block_at(&data, x, y, MATERIAL_BLOCK_RETRACTED_Z);
                            if let Some(lever) = lever_for_material(&material) {
                                println!("    material_block_spec(LeverKind::{:?}, {}, {}, false),", lever, x, y);
                                count += 1;
                            }
                        }
                    }
                }
            }
            println!("];");
            eprintln!("pattern {} total pieces: {}", p, count);
        }
    }

    #[test]
    #[ignore]
    fn diff_pattern_against_original() {
        let original = RoomData::from_raw_json(include_str!("../../room_data/rooms/65,water_board,-60,-60.json"));
        let raw_files: [&str; 4] = [
            include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern0,-60,-60.json"),
            include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern1,-60,-60.json"),
            include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern2,-60,-60.json"),
            include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern3,-60,-60.json"),
        ];

        for (p, raw) in raw_files.iter().enumerate() {
            let data = RoomData::from_raw_json(raw);
            let mut categories: HashMap<&str, usize> = HashMap::new();
            let mut other_names: HashMap<String, usize> = HashMap::new();
            let n = original.block_data.len().min(data.block_data.len());
            for i in 0..n {
                let a = &original.block_data[i];
                let b = &data.block_data[i];
                if a != b {
                    let cat = match b {
                        Blocks::StillWater { .. } | Blocks::FlowingWater { .. } => "water",
                        Blocks::Wool { .. } => "wool",
                        Blocks::RedstoneBlock | Blocks::Stone { .. } => "redstone_block_or_stone",
                        Blocks::StickyPiston { .. } | Blocks::PistonHead { .. } => "piston",
                        Blocks::CoalBlock | Blocks::GoldBlock | Blocks::QuartzBlock { .. }
                        | Blocks::DiamondBlock | Blocks::EmeraldBlock | Blocks::HardenedClay => "material",
                        Blocks::LapisLazuliBlock => "lapis",
                        Blocks::Lever { .. } => "lever",
                        Blocks::Air => "air",
                        _ => "other",
                    };
                    *categories.entry(cat).or_insert(0) += 1;
                    if cat == "other" {
                        let name = format!("{:?}", b);
                        let name = name.split(|c| c == '{' || c == ' ').next().unwrap_or("?").to_string();
                        *other_names.entry(name).or_insert(0) += 1;
                    }
                }
            }
            eprintln!("pattern {} vs original: {:?}", p, categories);
            eprintln!("pattern {} 'other' breakdown: {:?}", p, other_names);
        }
    }

    /// Verifies `apply_pattern` actually produces 4 pairwise-distinct `block_data` arrays (and
    /// the right `id` tag) when called with 0/1/2/3 on a fresh clone each time - a real check of
    /// the selection/swap logic itself, not just the source data (which `decode_material_block_patterns`
    /// already verified). Not `#[ignore]`d - cheap, no real-server dependency, safe to run as a
    /// normal part of `cargo test`.
    #[test]
    fn apply_pattern_produces_distinct_results() {
        let original = RoomData::from_raw_json(include_str!("../../room_data/rooms/65,water_board,-60,-60.json"));
        let mut results: Vec<(u8, String, Vec<Blocks>)> = Vec::new();
        for pattern in 0..4u8 {
            let mut data = original.clone();
            super::apply_pattern(&mut data, pattern);
            assert_eq!(data.id, format!("water_board_pattern_{pattern}"), "pattern {pattern} got the wrong id tag");
            results.push((pattern, data.id.clone(), data.block_data));
        }
        for i in 0..results.len() {
            for j in (i + 1)..results.len() {
                let (pi, _, bi) = &results[i];
                let (pj, _, bj) = &results[j];
                assert_ne!(bi, bj, "pattern {pi} and pattern {pj} produced IDENTICAL block_data - real bug");
            }
        }
        eprintln!("all 4 patterns produced distinct block_data, correctly tagged");
    }

    /// Odin's OWN real pattern-detection logic (`WaterSolver.kt::scan`, fetched directly from
    /// `github.com/odtheking/Odin` - not paraphrased from memory), reproduced exactly, in the
    /// exact same priority order (first match wins):
    ///   (14,77,27) == Terracotta      -> 0
    ///   (16,78,27) == EmeraldBlock    -> 1
    ///   (14,78,27) == DiamondBlock    -> 2
    ///   (14,78,27) == QuartzBlock     -> 3
    ///   else -> Odin fails outright ("Failed to get Water Board pattern")
    /// Checks what Odin would ACTUALLY detect, live in-game, for each of this project's 5 real
    /// room states (the original capture + patterns 0/1/3), and compares it against
    /// `answer_key_pattern_for_room`'s own assignment for that same state - a mismatch here means
    /// Odin's solver and this server's own `WATER_SOLUTIONS` lookup disagree on which pattern is
    /// active, silently pulling different answer-key timings for the same real room.
    /// One-off diagnostic (not a correctness assertion - purely investigative) for "Odin's solver
    /// sometimes doesn't work": scans every real captured Water Board room state (the original
    /// capture + all 4 pattern captures) for any real `Wool` block sitting anywhere near each of
    /// the 5 gates' own real geometry (`x` 10..=20, `y` 53..=58, at that gate's own `z`), to see
    /// whether any capture happens to show a gate closed at capture time - which would reveal the
    /// real wool color/metadata for `GateColor::Purple`/`Blue`, the 2 colors `wool_metadata`'s own
    /// doc comment says were never directly observed and are only a guessed vanilla-standard value.
    /// Run with `cargo test dump_gate_wool_colors -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn dump_gate_wool_colors() {
        let cases: [(&str, &str); 5] = [
            ("original", include_str!("../../room_data/rooms/65,water_board,-60,-60.json")),
            ("pattern_0", PATTERN_0_JSON),
            ("pattern_1", PATTERN_1_JSON),
            ("pattern_2", include_str!("../../room_data/misc/puzzles/Waterboard/65,water_board_pattern2,-60,-60.json")),
            ("pattern_3", PATTERN_3_JSON),
        ];
        let gates: [(&str, i32); 5] = [
            ("Purple", GateColor::Purple.local_pos().z),
            ("Orange", GateColor::Orange.local_pos().z),
            ("Blue", GateColor::Blue.local_pos().z),
            ("Green", GateColor::Green.local_pos().z),
            ("Red", GateColor::Red.local_pos().z),
        ];
        for (label, raw) in cases {
            let data = RoomData::from_raw_json(raw);
            for &(name, z) in &gates {
                let mut found = Vec::new();
                for x in 10..=20 {
                    for y in 53..=58 {
                        if let Blocks::Wool { color } = block_at(&data, x, y, z) {
                            found.push(format!("({x},{y},{z})=wool({color})"));
                        }
                    }
                }
                eprintln!("{label} gate {name} (z={z}): {}", if found.is_empty() { "no wool found (open)".to_string() } else { found.join(", ") });
            }
        }
    }

    #[test]
    #[ignore]
    fn check_odin_pattern_detection_matches() {
        fn odin_pattern(data: &RoomData) -> Option<i32> {
            if block_at(data, 14, 77, 27) == Blocks::HardenedClay {
                Some(0)
            } else if block_at(data, 16, 78, 27) == Blocks::EmeraldBlock {
                Some(1)
            } else if block_at(data, 14, 78, 27) == Blocks::DiamondBlock {
                Some(2)
            } else if matches!(block_at(data, 14, 78, 27), Blocks::QuartzBlock { .. }) {
                Some(3)
            } else {
                None
            }
        }

        let cases: [(&str, &str); 4] = [
            ("original (id \"-60,-60\")", include_str!("../../room_data/rooms/65,water_board,-60,-60.json")),
            ("water_board_pattern_0", PATTERN_0_JSON),
            ("water_board_pattern_1", PATTERN_1_JSON),
            ("water_board_pattern_3", PATTERN_3_JSON),
        ];
        let our_index: [&str; 4] = ["0", "1", "2", "3"]; // matches answer_key_pattern_for_room's own match arms, in this same order

        for ((label, raw), &ours) in cases.iter().zip(our_index.iter()) {
            let data = RoomData::from_raw_json(raw);
            let odin = odin_pattern(&data);
            eprintln!("{label}: Odin detects {odin:?}, we use answer key \"{ours}\" - {}",
                if odin.map(|o| o.to_string()) == Some(ours.to_string()) { "MATCH" } else { "MISMATCH" });
        }
    }
}

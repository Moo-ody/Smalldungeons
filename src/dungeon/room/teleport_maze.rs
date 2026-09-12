//! Teleport Maze puzzle, ported from real, verified sources rather than guessed:
//!
//! - The room's own captured layout (`72,teleport_maze,-60,-456.json`'s `block_data`) contains
//!   exactly 30 unfilled End Portal Frame blocks (block id 120, no "has eye" bit set) at y=69 -
//!   these are the real teleport pads, decoded directly the same way every other puzzle's fixed
//!   positions in this codebase were (see `creeper_beams.rs`'s module doc comment for the same
//!   technique).
//! - OdinClient's own real, decompiled solver (`TPMazeSolver.kt`, its `endPortalFrameLocations`
//!   field) hardcodes the exact same 30 room-relative positions - cross-checked 1:1 against the
//!   independently-decoded set above, confirming both this room's specific captured layout and
//!   which block type is the real pad.
//! - The Hypixel Wiki's own description of the room: 9 barred sections in a 3x3 layout (7
//!   interior ones each marked by a different vanilla ore block - gold/iron/coal/lapis/diamond/
//!   redstone/emerald, all independently found in this same `block_data` as 5-block "plus" marker
//!   clusters at y=68 - plus one entrance and one end section, each with exactly 1 pad instead of
//!   4); "each pad is linked to another teleport pad; standing on a specific one will always bring
//!   you to a same other one"; "after using a teleport pad, you face towards the teleporter that
//!   will take you to the final room."
//! - OdinClient's real solver doesn't hardcode a pad-link graph at all - it has no notion of
//!   which pad leads where. It instead watches the server's own forced look-direction packet
//!   that arrives right after each teleport (`tpPacket`'s `S08PacketPlayerPosLook` listener) and
//!   highlights whichever known pad position that direction happens to point at
//!   (`getCorrectPortals`'s `isXZInterceptable` raycast filter). That confirms the *real*
//!   mechanism is server-authoritative exactly like the rest of this codebase already is for
//!   every other puzzle: the server decides both the (fixed, per-dungeon) pad-to-pad link and
//!   the "correct direction" hint, and forces the client's camera to face it - the client mod is
//!   just reading that back, not computing it. This file is that server-side logic.
//!
//! Three pads have a fixed, non-random role, all confirmed in-game rather than guessed - the
//! other 27 are ordinary maze pads:
//!
//! - `ENTRANCE_PAD` (15, 69, 12) - where the player starts; stepping on it sends them to a random
//!   pad in one of the 7 ordinary chambers, same as every other pad's `dest` (see below).
//! - `EXIT_PAD` (15, 69, 14) - physically sits in the *reward room* (the same room the chest is
//!   in), not out in the maze proper - a player only ever encounters it after already reaching
//!   the reward chest. Reaching it always (a fixed override, not the random `dest` matching, and
//!   it never participates in that matching at all) sends the player back to `EXIT_DESTINATION` -
//!   "the start," facing north (confirmed, not toward `look_at_index` like every other teleport -
//!   this one's a plain shortcut, not a "keep going" step) - purely a convenience to leave and try
//!   again, with *no* effect on `chest_revealed`. Not the win condition (an earlier version of
//!   this file wrongly treated it as one).
//! - `look_at_index` - *not* a fixed pad, and not `EXIT_PAD` either. Reaching it (from anywhere,
//!   via the random `dest` matching like any other pad) reveals the reward chest and teleports
//!   the player to it, facing it - but confirmed, that alone does *not* solve the puzzle;
//!   *opening* the chest is the actual win condition (unlike, say, Creeper Beams, which completes
//!   on its own the instant its last correct pair locks in - see
//!   `DungeonSecret::puzzle_room_index`). Picked freshly at random each dungeon from the 27
//!   ordinary pads (excluding both `ENTRANCE_PAD` and `EXIT_PAD`). It's also what the "face
//!   towards the teleporter that will take you to the final room" quote above describes: on
//!   *every* teleport except the `EXIT_PAD` -> `EXIT_DESTINATION` hop, the forced look direction
//!   always points at this same pad.
//!
//! One thing is generated per dungeon (`build_maze_graph`, called from `setup`): `dest`, a perfect
//! matching over all 30 pads (15 symmetric pairs, `EXIT_PAD` included - see `build_maze_graph`'s
//! own doc comment for why it has to be, even though its own outgoing link is never read) that
//! never pairs two pads in the *same* physical chamber (`SECTION_OF`, recovered by flood-filling
//! this room's own `block_data` - see its doc comment) - stepping on pad `i` physically teleports
//! you to pad `dest[i]` in a *different* chamber, and vice versa, otherwise directionless (a wrong
//! pad doesn't strand you, it just relocates you somewhere else in the maze - the forced look is
//! what actually guides solving). Its *connectivity backbone* is a single random path visiting
//! every chamber but `EXIT_PAD`'s exactly once, purely to *guarantee* every chamber can eventually
//! reach `look_at_index`'s chamber by some sequence of teleports - see `build_maze_graph`'s own
//! doc comment for the full construction.
//!
//! The reward chest at (15, 70, 20) - real, already part of this room's static layout, and
//! confirmed to be what real Hypixel actually calls a "Puzzle Chest" specifically, not a generic
//! secret chest - becomes a genuine secret chest here anyway (`DungeonSecret`/`SecretType::Chest`,
//! the same mechanism every other dungeon secret chest uses - there's no behavioral difference
//! that would need its own separate code path) only once `look_at_index` is reached
//! (`reveal_reward_chest` clears the static block and spawns a fresh one in its place) - it's
//! physically unreachable before that anyway (the small reward room has no walkable connection to
//! the maze, only the reward teleport reaches it), so there's no regression in leaving it
//! non-interactable until then. Confirmed: reaching it only reveals the chest - *opening* it is
//! the actual win condition (`DungeonSecret::puzzle_room_index`), unlike e.g. Creeper Beams, which
//! completes on its own. It also carries a "blessing" - see `SOLVE_BLESSING_TEXTURE`'s doc
//! comment.

use crate::dungeon::room::room::Room;
use crate::dungeon::room::secrets::{DungeonSecret, SecretType};
use crate::net::protocol::play::clientbound::{Particles, SoundEffect};
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::entity::dungeon_mobs::ai::movement;
use crate::server::player::player::ClientId;
use crate::server::utils::direction::Direction;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::particles::ParticleTypes;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;
use crate::utils::seeded_rng::seeded_rng;
use rand::prelude::SliceRandom;
use rand::Rng;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// All 30 real teleport pad positions (room-relative, pre-rotation) - see the module doc comment
/// for how these were recovered and cross-checked.
const PAD_POSITIONS: [BlockPos; 30] = [
    BlockPos { x: 4, y: 69, z: 6 }, BlockPos { x: 4, y: 69, z: 12 }, BlockPos { x: 4, y: 69, z: 14 },
    BlockPos { x: 4, y: 69, z: 20 }, BlockPos { x: 4, y: 69, z: 22 }, BlockPos { x: 4, y: 69, z: 28 },
    BlockPos { x: 10, y: 69, z: 6 }, BlockPos { x: 10, y: 69, z: 12 }, BlockPos { x: 10, y: 69, z: 14 },
    BlockPos { x: 10, y: 69, z: 20 }, BlockPos { x: 10, y: 69, z: 22 }, BlockPos { x: 10, y: 69, z: 28 },
    BlockPos { x: 12, y: 69, z: 22 }, BlockPos { x: 12, y: 69, z: 28 },
    BlockPos { x: 15, y: 69, z: 12 }, BlockPos { x: 15, y: 69, z: 14 },
    BlockPos { x: 18, y: 69, z: 22 }, BlockPos { x: 18, y: 69, z: 28 },
    BlockPos { x: 20, y: 69, z: 6 }, BlockPos { x: 20, y: 69, z: 12 }, BlockPos { x: 20, y: 69, z: 14 },
    BlockPos { x: 20, y: 69, z: 20 }, BlockPos { x: 20, y: 69, z: 22 }, BlockPos { x: 20, y: 69, z: 28 },
    BlockPos { x: 26, y: 69, z: 6 }, BlockPos { x: 26, y: 69, z: 12 }, BlockPos { x: 26, y: 69, z: 14 },
    BlockPos { x: 26, y: 69, z: 20 }, BlockPos { x: 26, y: 69, z: 22 }, BlockPos { x: 26, y: 69, z: 28 },
];

/// Where the player starts - confirmed in-game (see the module doc comment for the full role
/// breakdown). One of the two singleton-chamber pads sharing the room's center column, x=15.
const ENTRANCE_PAD: BlockPos = BlockPos { x: 15, y: 69, z: 12 };

/// The other singleton-chamber pad on the center column - confirmed in-game as sitting in the
/// reward room (with the chest), *not* out in the maze, and *not* the win condition - see the
/// module doc comment. Reaching it always sends the player to `EXIT_DESTINATION`.
const EXIT_PAD: BlockPos = BlockPos { x: 15, y: 69, z: 14 };

/// Room-relative feet position `EXIT_PAD` always sends the player to - "the start," confirmed
/// in-game. Not another pad (there's no real block there in this room's own captured layout), a
/// small fixed platform near the entrance.
const EXIT_DESTINATION: BlockPos = BlockPos { x: 15, y: 69, z: 9 };

/// Room-relative feet position the *reward* teleport (reaching `look_at_index`, the real win
/// condition) actually lands the player at, confirmed in-game - not another pad, a small fixed
/// platform right in front of the reward chest.
const REWARD_LANDING: BlockPos = BlockPos { x: 15, y: 68, z: 17 };

/// Room-relative position of the reward chest - confirmed in-game, and independently found
/// already sitting in this room's own static `block_data` (block id 54, "Chest") at exactly this
/// position. Once revealed, `reveal_reward_chest` clears whatever's there and places a fresh one
/// facing north, registered as a real `DungeonSecret` chest instead of an inert decorative block - see
/// the module doc comment.
const EXISTING_CHEST: BlockPos = BlockPos { x: 15, y: 70, z: 20 };

/// Base64 skull `Value` for the reward chest's "blessing" - a real captured player skin, given
/// directly (not derived/guessed). `DungeonSecret::blessing_texture`'s doc comment covers what
/// this actually triggers on open (the ascending `note.harp` sequence + floating skull, reusing
/// `EssenceEntityImpl` - see the `Chest` interact handler in `block_interact_action.rs`).
const SOLVE_BLESSING_TEXTURE: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZTkzZTIwNjg2MTc4NzJjNTQyZWNkYTFkMjdkZjRlY2U5MWM2OTk5MDdiZjMyN2M0ZGRiODUzMDk0MTJkMzkzOSJ9fX0=";

#[derive(Debug)]
pub struct TeleportMazeState {
    room_index: usize,
    /// World positions, index-aligned with `PAD_POSITIONS`/`dest`.
    pads: Vec<BlockPos>,
    /// The reward-room pad that always sends the player back to `exit_destination` - see
    /// `EXIT_PAD`. Not the win condition - see `look_at_index`.
    exit_index: usize,
    /// World feet position `exit_index` always sends the player to - see `EXIT_DESTINATION`.
    exit_destination: BlockPos,
    /// This room's own rotation - needed to turn the room-relative "face north" the exit
    /// teleport wants (see `tick_player`'s `EXIT_PAD` branch) into a real world yaw, the same way
    /// `three_weirdos.rs` rotates its NPCs' spawn facing.
    rotation: Direction,
    /// The actual win condition, and the pad the player's look is always forced toward on every
    /// teleport - a random pick from the 27 ordinary pads (never `exit_index`, never the entrance
    /// pad), fresh each dungeon. See the module doc comment.
    look_at_index: usize,
    /// World feet position the reward teleport lands you at, and the world position of the
    /// pre-existing reward chest to face once there - see `REWARD_LANDING`/`EXISTING_CHEST`.
    reward_landing: BlockPos,
    chest_pos: BlockPos,
    /// `dest[i]` = index of the pad stepping on pad `i` physically teleports you to (symmetric).
    dest: Vec<usize>,
    /// Whether the reward chest has already been revealed (a real `DungeonSecret` spawned in
    /// place of the room's static block) - *not* whether the puzzle is solved, which now only
    /// happens when that chest is actually opened (see `DungeonSecret::puzzle_room_index`). Just
    /// prevents `reveal_reward_chest` from re-running (and re-placing an un-opened chest over
    /// one that may already have been grabbed) on a second visit to `look_at_index`.
    chest_revealed: bool,
    /// Last pad index each tracked player was standing on (`None` = not on any pad right now) -
    /// edge-triggered so landing on a pad fires its teleport exactly once, on the tick they
    /// actually step onto it, not every tick they happen to still be standing there (which would
    /// otherwise immediately re-fire the *next* pad's own teleport too, since arriving lands you
    /// on another pad by definition).
    player_pad: HashMap<ClientId, Option<usize>>,
    /// World tick each tracked player was last teleported (pad-to-pad or the reward hop) - see
    /// `TELEPORT_GRACE_TICKS`.
    last_teleport_tick: HashMap<ClientId, u64>,
}

/// Originally written because a stale, in-flight-before-the-teleport client position packet
/// could silently snap `player.position` straight back to the pad just left, which the very next
/// tick's edge-detection then read as "stepped onto that old pad again," instantly re-firing its
/// own teleport - the "insta teleports me back" bug. That root cause is now fixed at the shared
/// `Player::server_teleport`/`pending_teleport` level (see `PendingTeleport`'s doc comment) -
/// stale reports are rejected there directly rather than relying on a timing window here. This
/// grace period is kept anyway as harmless defense-in-depth (skipping the pad re-check for a
/// moment after landing costs nothing a deliberate immediate walk-off-and-back-on would notice),
/// not because it's still load-bearing for that bug specifically. The forced look direction, by
/// contrast, is a real one-shot: set once at the moment of landing and never touched again - the
/// player can freely look around immediately afterward, it doesn't fight their mouse or get
/// re-imposed.
const TELEPORT_GRACE_TICKS: u64 = 10;

/// Spawns the Teleport Maze puzzle's pad graph for `room` if it actually is one - no-op for
/// every other room. Called once from `Room::load_into_world`, same timing/pattern as
/// `three_weirdos::setup`/`creeper_beams::setup`. Needs `&mut Room` (unlike those two) so the
/// generated state can live directly on `room.teleport_maze_state` - there's no
/// `BlockInteractAction`-based interactable here to hang it off of instead, since a teleport pad
/// triggers by being *walked onto*, not right-clicked (see `tick`).
pub fn setup(room: &mut Room, room_index: usize, world: &World) {
    if room.room_data.name != "Teleport Maze" {
        return;
    }

    let pads: Vec<BlockPos> = PAD_POSITIONS.iter().map(|p| room.get_world_block_pos(p)).collect();
    let exit_world = room.get_world_block_pos(&EXIT_PAD);
    let Some(exit_index) = pads.iter().position(|&p| p == exit_world) else { return };
    let entrance_world = room.get_world_block_pos(&ENTRANCE_PAD);
    let Some(entrance_index) = pads.iter().position(|&p| p == entrance_world) else { return };
    let exit_destination = room.get_world_block_pos(&EXIT_DESTINATION);
    let reward_landing = room.get_world_block_pos(&REWARD_LANDING);
    let chest_pos = room.get_world_block_pos(&EXISTING_CHEST);

    let _ = world; // not needed yet - kept as a parameter for symmetry with the other puzzle setups

    // A fresh random pick each dungeon, excluding both fixed pads - see `look_at_index`'s doc
    // comment on `TeleportMazeState`. Computed before `build_maze_graph` since it needs this as
    // the connectivity backbone's actual goal (not `exit_index` - see there).
    let look_at_index = {
        let mut rng = seeded_rng();
        let candidates: Vec<usize> = (0..pads.len()).filter(|&i| i != exit_index && i != entrance_index).collect();
        candidates[rng.random_range(0..candidates.len())]
    };

    let dest = build_maze_graph(pads.len(), look_at_index, exit_index);

    room.teleport_maze_state = Some(Rc::new(RefCell::new(TeleportMazeState {
        room_index,
        pads,
        exit_index,
        exit_destination,
        rotation: room.rotation,
        look_at_index,
        reward_landing,
        chest_pos,
        dest,
        chest_revealed: false,
        player_pad: HashMap::new(),
        last_teleport_tick: HashMap::new(),
    })));
}

/// Which of the 9 physical chambers (see the module doc comment) each `PAD_POSITIONS` index
/// belongs to - recovered by flood-filling this room's own `block_data` (walkable columns at
/// player foot/head height, y=70/71) rather than guessed from coordinates, since the chambers
/// aren't a clean geometric 3x3 grid. Confirms the wiki's "7 four-pad chambers + 1 one-pad
/// entrance + 1 one-pad end" structure exactly: sections 0,1,2,5,6,7,8 have 4 members each,
/// section 3 (`ENTRANCE_SECTION`, containing `PAD_POSITIONS[14]` = `ENTRANCE_PAD`) and section 4
/// (containing `PAD_POSITIONS[15]` = `EXIT_PAD`) have exactly 1 each.
const SECTION_OF: [u8; 30] = [
    0, 0, 1, 1, 2, 2, 0, 0, 1, 1, 2, 2, 5, 5, 3, 4, 5, 5, 6, 6, 7, 7, 8, 8, 6, 6, 7, 7, 8, 8,
];
const NUM_SECTIONS: usize = 9;
/// The one-pad section you start in - always the first stop on `build_maze_graph`'s random path,
/// so it's only ever the *parent* of the path's second chamber (never a later one), and a
/// teleport can never send you back to it.
const ENTRANCE_SECTION: u8 = 3;

/// Builds `dest` (see the module doc comment) - a random perfect matching over all 30 pads that
/// never pairs two pads in the same physical chamber. `EXIT_PAD`'s own *chamber* (`exclude_index`,
/// a one-pad chamber - `SECTION_OF` - that can't supply the two distinct pads an interior path
/// node needs) is kept out of the connectivity backbone below, but `EXIT_PAD` itself still gets a
/// real `dest` partner via the ordinary leftover-pairing pass at the end, same as any other pad -
/// deliberately *not* excluded from the matching outright: 30 is even, but 29 (30 minus just
/// `EXIT_PAD`) isn't, and an odd-sized pool can never pair off completely (`chunks_exact(2)`
/// silently drops the last one, leaving its `dest` entry unset - a real crash this function used
/// to have). `EXIT_PAD`'s own *outgoing* `dest` link is simply never read - `tick_player` checks
/// for it before ever consulting `dest` - so which pad ends up linked to it doesn't matter, only
/// that every pad has *some* partner.
///
/// The backbone itself is a single random path visiting every chamber but `EXIT_PAD`'s exactly
/// once (entrance first, `goal_index`'s chamber last, the remaining chambers shuffled in between)
/// purely to *guarantee* the puzzle is solvable: without some connectivity guarantee, a plain
/// same-chamber-avoiding random matching could (rarely, but possibly) end up as several
/// disconnected clusters of chambers with no link between them at all. Each adjacent pair on the
/// path gets linked by one pad in each chamber (`dest[pad_a] = pad_b`, symmetric) - that alone
/// guarantees every chamber can reach `goal_index`'s chamber (`look_at_index`, the real win
/// condition - see the module doc comment) by *some* sequence of teleports (walking to any of a
/// chamber's other pads once there is always physically possible, since all of a chamber's pads
/// share one open floor).
fn build_maze_graph(count: usize, goal_index: usize, exclude_index: usize) -> Vec<usize> {
    let mut rng = seeded_rng();
    let goal_section = SECTION_OF[goal_index] as usize;
    let entrance_section = ENTRANCE_SECTION as usize;
    let exclude_section = SECTION_OF[exclude_index] as usize;

    let mut section_pads: Vec<Vec<usize>> = vec![Vec::new(); NUM_SECTIONS];
    for i in 0..count {
        section_pads[SECTION_OF[i] as usize].push(i);
    }

    let mut dest = vec![usize::MAX; count];
    let mut used = vec![false; count];

    let mut middle: Vec<usize> = (0..NUM_SECTIONS)
        .filter(|&s| s != goal_section && s != entrance_section && s != exclude_section)
        .collect();
    middle.shuffle(&mut rng);
    let mut path = vec![entrance_section];
    path.extend(middle);
    path.push(goal_section);

    for i in 1..path.len() {
        let section = path[i];
        let predecessor = path[i - 1];
        let a_options: Vec<usize> = section_pads[section].iter().copied().filter(|&p| !used[p]).collect();
        let a = a_options[rng.random_range(0..a_options.len())];
        let b_options: Vec<usize> = section_pads[predecessor].iter().copied().filter(|&p| !used[p]).collect();
        let b = b_options[rng.random_range(0..b_options.len())];

        used[a] = true;
        used[b] = true;
        dest[a] = b;
        dest[b] = a;
    }

    // Pair off every remaining (not yet linked) pad, still avoiding same-chamber pairs. Random
    // greedy with a bounded retry - virtually always succeeds immediately given how few leftovers
    // land in any one chamber (at most 3) relative to the whole leftover pool.
    let leftover: Vec<usize> = (0..count).filter(|&p| !used[p]).collect();
    let mut paired = false;
    for _ in 0..2000 {
        let mut pool = leftover.clone();
        pool.shuffle(&mut rng);
        let mut attempt = vec![None; count];
        let mut ok = true;
        while let Some(a) = pool.pop() {
            let Some(idx) = pool.iter().position(|&b| SECTION_OF[b] != SECTION_OF[a]) else {
                ok = false;
                break;
            };
            let b = pool.remove(idx);
            attempt[a] = Some(b);
            attempt[b] = Some(a);
        }
        if ok {
            for (i, v) in attempt.into_iter().enumerate() {
                if let Some(b) = v {
                    dest[i] = b;
                }
            }
            paired = true;
            break;
        }
    }
    if !paired {
        // Astronomically unlikely given the numbers above - last-resort fallback so this can't
        // loop or panic: pair whatever's left in order, ignoring the same-chamber rule.
        for pair in leftover.chunks_exact(2) {
            dest[pair[0]] = pair[1];
            dest[pair[1]] = pair[0];
        }
    }

    dest
}

/// Called every tick from `Room::tick` for every room (a no-op for every room but the one whose
/// `teleport_maze_state` is `Some`, i.e. at most one per dungeon).
pub fn tick(room: &mut Room, world: &mut World) {
    let Some(state_rc) = room.teleport_maze_state.clone() else { return };

    // Deliberately never gated on any "is this all done" flag - `EXIT_PAD` (only ever reachable
    // in the reward room, after `look_at_index` has already been reached) needs `tick_player` to
    // keep running indefinitely or it could never be detected at all, and reaching `look_at_index`
    // more than once is expected/harmless (re-teleports to the reward chest each time,
    // `chest_revealed` already keeps that idempotent on the one part that actually needs it - see
    // `teleport_to_reward_chest`).
    let client_ids: Vec<ClientId> = world.players.keys().copied().collect();
    for client_id in client_ids {
        tick_player(&state_rc, world, client_id);
    }
}

/// Yaw to look from `from_feet` (a landing spot) directly at `target_pos`'s block center (+0.5
/// on x/z, horizontal only). Per explicit request, every forced look direction this puzzle sets
/// on a teleport keeps pitch level at 0 - callers no longer get a pitch out of this at all, only
/// the yaw.
fn look_at_direction(from_feet: DVec3, target_pos: BlockPos) -> f32 {
    let dx = (target_pos.x as f64 + 0.5) - from_feet.x;
    let dz = (target_pos.z as f64 + 0.5) - from_feet.z;
    movement::yaw_towards(dx, dz)
}

/// A real teleport pad sits at one corner of a small 2x2 raised platform - the pad's own cell
/// plus 2 orthogonal `StoneSlab` neighbors and one diagonal `StoneSlab` corner cell, confirmed
/// directly in this room's own captured `block_data` (every `PAD_POSITIONS` entry checked by
/// hand). Landing on a pad via the ordinary pad-to-pad teleport lands the player on that
/// diagonal corner slab instead of centered on the pad itself, per explicit request - real
/// Hypixel does the same (you never end up standing dead-center on the frame block you teleport
/// onto).
///
/// Ordinary chamber pads have exactly one valid diagonal (2 solid walls + 1 slab-lined corner);
/// the two single-pad chambers (the entrance and the reward-room's `EXIT_PAD`) instead have slabs
/// on 3 of their 4 sides, leaving *two* geometrically valid diagonal corners with no single
/// "correct" one recoverable from the room's static layout alone. For that rare case, this picks
/// whichever valid corner's direction from the pad best matches the direction toward
/// `look_at_pos` (the same target the yaw is already being pointed at) - a reasonable tie-break
/// reusing data already being computed, not a guess independent of anything else known here.
/// Returns `(corner position, whether that slab is the upper half)` - the upper half needs `+1.0`
/// under the player's feet like a full block, the lower half only `+0.5`.
fn find_corner_slab(world: &World, pad: BlockPos, look_at_pos: BlockPos) -> Option<(BlockPos, bool)> {
    let mut valid: Vec<(BlockPos, bool)> = Vec::new();
    for (dx, dz) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
        let corner = BlockPos { x: pad.x + dx, y: pad.y, z: pad.z + dz };
        if let Blocks::StoneSlab { top_half, .. } = world.get_block_at(corner.x, corner.y, corner.z) {
            valid.push((corner, top_half));
        }
    }

    if valid.len() <= 1 {
        return valid.into_iter().next();
    }

    let target_dx = (look_at_pos.x - pad.x) as f64;
    let target_dz = (look_at_pos.z - pad.z) as f64;
    valid.into_iter().max_by(|(a, _), (b, _)| {
        let score = |p: &BlockPos| ((p.x - pad.x) as f64) * target_dx + ((p.z - pad.z) as f64) * target_dz;
        score(a).partial_cmp(&score(b)).unwrap()
    })
}

fn tick_player(state_rc: &Rc<RefCell<TeleportMazeState>>, world: &mut World, client_id: ClientId) {
    let current_tick = world.tick_count;
    let Some(player) = world.players.get(&client_id) else { return };

    // See `TELEPORT_GRACE_TICKS`'s doc comment - `player.position` right after a teleport can't be
    // trusted for a short window (a stale in-flight client packet can silently snap it straight
    // back to the pad they just left), so just skip the pad re-check entirely this tick rather
    // than act on it.
    {
        let state = state_rc.borrow();
        if let Some(&last) = state.last_teleport_tick.get(&client_id) {
            if current_tick.saturating_sub(last) < TELEPORT_GRACE_TICKS {
                return;
            }
        }
    }

    let feet_block = BlockPos::new(
        player.position.x.floor() as i32,
        (player.position.y - 0.1).floor() as i32,
        player.position.z.floor() as i32,
    );

    let pad_index = {
        let mut state = state_rc.borrow_mut();
        let current = state.pads.iter().position(|&p| p == feet_block);
        let previous = state.player_pad.get(&client_id).copied().flatten();
        if current == previous {
            return;
        }
        state.player_pad.insert(client_id, current);
        let Some(pad_index) = current else { return };
        pad_index
    };

    // `EXIT_PAD` has its own fixed, unconditional override - it never consults `dest` (its own
    // entry there is never read - see `build_maze_graph`) and has no effect on `chest_revealed`.
    // Checked before anything else, same as `is_win` needs to be checked before consulting `dest`
    // for any other pad - both are "don't route this one through the matching" cases.
    if pad_index == state_rc.borrow().exit_index {
        let (exit_destination, rotation) = {
            let state = state_rc.borrow();
            (state.exit_destination, state.rotation)
        };
        // +0, not +1 like every other landing here - confirmed in-game this one's a level lower.
        let landing_feet = DVec3::new(exit_destination.x as f64 + 0.5, exit_destination.y as f64, exit_destination.z as f64 + 0.5);
        // Confirmed: faces north (room-relative, so rotated by this room's own orientation the
        // same way `three_weirdos.rs` rotates its NPC spawn facing) - not toward `look_at_index`
        // like every other teleport. This one's a fixed shortcut back to the start, not a step
        // that needs the "keep going" compass cue.
        let (yaw, pitch) = (180.0_f32.rotate(rotation), 0.0);

        state_rc.borrow_mut().last_teleport_tick.insert(client_id, current_tick);
        let Some(player) = world.players.get_mut(&client_id) else { return };
        player.server_teleport(landing_feet, yaw, pitch, 0);
        play_teleport_effects(player, landing_feet);
        return;
    }

    let look_at_index = state_rc.borrow().look_at_index;
    let (landing_index, is_win) = if pad_index == look_at_index {
        (pad_index, true)
    } else {
        let landing = state_rc.borrow().dest[pad_index];
        (landing, landing == look_at_index)
    };

    if pad_index != landing_index {
        // Only actually happens when `pad_index` isn't itself the goal - mark them as standing
        // on `landing_index` now, otherwise the very next tick's position check would see them
        // "newly" on it (they never left) and immediately re-fire its teleport too.
        state_rc.borrow_mut().player_pad.insert(client_id, Some(landing_index));
    }

    if is_win {
        teleport_to_reward_chest(state_rc, world, client_id);
        return;
    }

    let landing_pos = state_rc.borrow().pads[landing_index];
    // `landing_index` can never equal `look_at_index` here (that's `is_win`, already handled and
    // returned above), so there's always a real, different pad to point the camera at.
    let look_at_pos = state_rc.borrow().pads[look_at_index];

    // The one-tick "Odin-detection" position: y specifically 69.5 (pad block y + 0.5), x/z on the
    // 0.5 grid - matches OdinClient's own solver (`TPMazeSolver.tpPacket`), which only recognizes
    // a `PlayerPosLook` as a pad-landing packet when `y == 69.5 && x % 0.5 == 0.0 && z % 0.5 ==
    // 0.0`. That y is *below* an End Portal Frame's real collision top (13/16 - a full block, not
    // the half this sends), so sent alone it visibly sinks the player partway into the block.
    // Sent first and immediately followed by the real resting position below, Odin's one-shot
    // listener still sees the exact packet it's watching for (it doesn't care what comes after) -
    // still on the pad's own cell, deliberately not the corner: this position only exists for the
    // detection packet, never actually seen/stood on by the player.
    let odin_feet = DVec3::new(landing_pos.x as f64 + 0.5, landing_pos.y as f64 + 0.5, landing_pos.z as f64 + 0.5);

    // Land on the pad's diagonal corner slab, not centered on the pad itself - see
    // `find_corner_slab`'s doc comment. Falls back to the pad's own center (the old behavior)
    // only if a pad is somehow missing its corner slab, which shouldn't happen for any real
    // captured pad.
    let landing_feet = match find_corner_slab(world, landing_pos, look_at_pos) {
        Some((corner, top_half)) => {
            let y = corner.y as f64 + if top_half { 1.0 } else { 0.5 };
            DVec3::new(corner.x as f64 + 0.5, y, corner.z as f64 + 0.5)
        }
        None => DVec3::new(landing_pos.x as f64 + 0.5, landing_pos.y as f64 + 1.0, landing_pos.z as f64 + 0.5),
    };
    // Yaw points from the real landing spot (the corner) toward the next hint; pitch stays level
    // per explicit request - every forced look direction here keeps pitch at 0.
    let yaw = look_at_direction(landing_feet, look_at_pos);
    let pitch = 0.0;

    state_rc.borrow_mut().last_teleport_tick.insert(client_id, current_tick);

    let Some(player) = world.players.get_mut(&client_id) else { return };
    // Two teleports back to back, deliberately: `odin_feet` first (purely for OdinClient's
    // one-shot pad-landing detection, see the comment above), then immediately superseded by the
    // real `landing_feet`. `server_teleport` handles this exactly right on its own - the second
    // call simply replaces the first as the newest expected destination, so only `landing_feet`
    // is ever actually waited on for acknowledgement; an echo for the throwaway `odin_feet` (if
    // the client even sends one before receiving the second packet) just won't match it and gets
    // harmlessly rejected.
    player.server_teleport(odin_feet, yaw, pitch, 0);
    player.server_teleport(landing_feet, yaw, pitch, 0);
    play_teleport_effects(player, landing_feet);
}

/// The real Hypixel dungeon teleport-pad sound (`mob.endermen.portal`) plus a burst of portal
/// particles at the landing spot.
fn play_teleport_effects(player: &mut crate::server::player::player::Player, pos: DVec3) {
    player.write_packet(&SoundEffect {
        sound: Sounds::EndermenPortal.id(),
        pos_x: pos.x,
        pos_y: pos.y,
        pos_z: pos.z,
        volume: 1.0,
        pitch: 1.0,
    });
    player.write_packet(&Particles {
        particle_id: ParticleTypes::Portal.get_id(),
        long_distance: true,
        x: pos.x as f32,
        y: pos.y as f32 + 0.5,
        z: pos.z as f32,
        offset_x: 0.3,
        offset_y: 0.5,
        offset_z: 0.3,
        speed: 0.0,
        count: 20,
    });
}

/// Reaching `look_at_index` triggers this: teleports the player to the fixed reward platform
/// (`REWARD_LANDING`), facing the reward chest, and reveals that chest for real the first time
/// (`chest_revealed` makes the reveal idempotent - a second visit still teleports the player
/// there, it just doesn't re-place a fresh un-opened chest over one that may already be open).
/// Confirmed in-game: this does *not* solve the puzzle by itself - see `reveal_reward_chest`.
fn teleport_to_reward_chest(state_rc: &Rc<RefCell<TeleportMazeState>>, world: &mut World, client_id: ClientId) {
    let (room_index, reward_landing, chest_pos, already_revealed) = {
        let mut state = state_rc.borrow_mut();
        let already_revealed = state.chest_revealed;
        state.chest_revealed = true;
        (state.room_index, state.reward_landing, state.chest_pos, already_revealed)
    };

    // +1 - `reward_landing` is the confirmed-in-game block coordinate; the player's feet land one
    // level above it (standing on top of that block), same as every pad landing above.
    let landing_feet = DVec3::new(reward_landing.x as f64 + 0.5, reward_landing.y as f64 + 1.0, reward_landing.z as f64 + 0.5);
    let yaw = look_at_direction(landing_feet, chest_pos);

    if let Some(player) = world.players.get_mut(&client_id) {
        player.server_teleport(landing_feet, yaw, 0.0, 0);
        play_teleport_effects(player, landing_feet);
    }

    if !already_revealed {
        reveal_reward_chest(world, room_index, chest_pos);
    }
}

/// Clears whatever's at `chest_pos` (the room's own static chest block) and replaces it with a
/// freshly-registered `DungeonSecret` - real secret-chest mechanics (open animation, sound, the
/// flame tell every secret chest gets) instead of an inert decorative block. Carries the blessing
/// (`SOLVE_BLESSING_TEXTURE`) and `puzzle_room_index` - the latter is what actually marks this
/// room done and broadcasts the solved message, but only once a player opens this chest (see the
/// `Chest` interact handler in `block_interact_action.rs`) - confirmed that's the puzzle's real
/// win condition, not merely reaching it.
fn reveal_reward_chest(world: &mut World, room_index: usize, chest_pos: BlockPos) {
    world.set_block_at(Blocks::Air, chest_pos.x, chest_pos.y, chest_pos.z);
    let rotation = world.server_mut().dungeon.rooms[room_index].rotation;
    let secret_rc = Rc::new(RefCell::new(DungeonSecret::new(SecretType::Chest { direction: Direction::North.rotate(rotation) }, chest_pos, 0.0)));
    {
        let mut secret = secret_rc.borrow_mut();
        secret.blessing_texture = Some(SOLVE_BLESSING_TEXTURE);
        secret.puzzle_room_index = Some(room_index);
    }
    let secret_ref = secret_rc.borrow_mut();
    DungeonSecret::spawn_into_world(&secret_rc, secret_ref, world);
}

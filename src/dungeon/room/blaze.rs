//! Higher or Lower puzzle - real room names "Lower Blaze"/"Higher Blaze" (internal
//! `room_data.name` is just "Blaze" for both; the two are distinguished by `room_data.bottom`,
//! 15 vs 65 - this project's own room-data model doesn't split them into two named rooms the way
//! Odin does, and there's no need to change that, just to branch on `bottom` here).
//!
//! 10 Blazes spawn at fixed positions (confirmed via live `/blazepuzzle` capture - 3 separate
//! captures of "Higher Blaze" produced byte-identical positions every time, only HP differed) and
//! must be killed in strict HP order - lowest-to-highest for Higher Blaze, highest-to-lowest for
//! Lower Blaze (per explicit request, cross-checked three ways: Odin's own `BlazeSolver.kt` sort
//! direction per room name, a direct real-game confirmation that entering from the top door means
//! killing highest-to-lowest, and Lower Blaze's captured room-relative Y range sitting exactly 50
//! below Higher Blaze's for the identical X/Z footprint - all three agree on the same mapping).
//! Any kill out of order fails the puzzle outright (per explicit request) - the remaining blazes
//! stay killable afterward (this is also each room's normal mob-clear requirement, not purely
//! optional), just without further order enforcement once already failed.
//!
//! HP itself uses Hypixel's own real formula (SkyBlock Wiki, fetched directly): the first blaze is
//! `rand(0,1000) + 1000` (1,000-1,999), and each subsequent one is `previous + rand(0,500) + 1` -
//! strictly increasing. Those 10 values are generated once, then shuffled across the 10 fixed
//! positions, so position alone never reveals the correct order - only reading each nametag does,
//! same as the real puzzle.
//!
//! Nametag format matches Odin's own confirmed detection regex exactly (`BlazeSolver.kt`:
//! `"^\[Lv15] Blaze [\d,]+/([\d,]+)❤$"` - a literal SINGLE space after the level bracket, the
//! level hardcoded to the literal text "Lv15" rather than any digit pattern, full comma-grouped
//! numbers, nothing trailing the heart) so any real solver mod reading this server's own nametags
//! would parse them exactly the same way it parses real Hypixel's. An earlier version of this used
//! a double space here - direct re-reading of the real `BlazeSolver.kt` source (not just an
//! earlier live-capture impression of it) confirmed that was wrong, and was the actual cause of
//! the solver not matching. Also confirmed by re-reading: Odin matches on `entity.name.noControlCodes`,
//! which strips every `§x` formatting code before the regex ever runs - so color codes in this
//! nametag were never actually the problem, only the extra space was. The level shown (`Lv15`) is
//! the exact value seen on every live-captured nametag, not a guess. The health text lives on a
//! separate following-armor-stand nametag entity (see `spawn_following_nametag`), not the Blaze
//! mob's own name - confirmed live via `/blazepuzzle debug`: the real mob entity's own name is
//! just plain "Blaze", the "[Lv15] Blaze N/N❤" text is a distinct nearby `ArmorStand`.
//!
//! Combat model: like every other dungeon mob in this project (`combat::apply_lethal_hit`), there
//! is no real incremental damage/health-depletion system - any weapon hit is an instant kill. That
//! matches the real captured data too: every single captured nametag showed current HP exactly
//! equal to max HP, never partially damaged, consistent with blazes always dying in one hit.

use crate::dungeon::room::room::Room;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::SoundEffect;
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::server::entity::dungeon_mobs::ai::combat;
use crate::server::entity::entity::{Entity, EntityId, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::entity::spawn_equipped::spawn_following_nametag;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::player::player::{ClientId, Player};
use crate::server::utils::direction::Direction;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;
use crate::utils::seeded_rng::seeded_rng;
use std::cell::RefCell;
use std::rc::Rc;

/// `room_data.bottom` values that distinguish the two physical Blaze rooms - see the module doc
/// comment for why this project doesn't split them into two `room_data.name`s the way Odin does.
/// `bottom == 65` is "Higher Blaze", `bottom == 15` is "Lower Blaze" - confirmed live (per direct
/// correction after a brief, mistaken revert of this exact swap). Both the position footprint and
/// the kill-order direction downstream are keyed off these same two constants, so this one pairing
/// fixes both together.
pub const HIGHER_BLAZE_BOTTOM: i32 = 65;
pub const LOWER_BLAZE_BOTTOM: i32 = 15;

/// Room-relative (canonical, pre-rotation) positions for the 10 blazes - confirmed fixed via live
/// capture (see the module doc comment). Higher and Lower Blaze share the identical X/Z for every
/// entry, differing only by a constant +50 on Y (Lower's own values, i.e. these two arrays are
/// kept as separately-captured ground truth rather than deriving one from the other by that
/// offset, since it's real captured data, not a derived assumption).
const HIGHER_BLAZE_POSITIONS: [(i32, i32, i32); 10] = [
    (15, 77, 13), (17, 105, 15), (15, 85, 17), (13, 113, 15), (17, 89, 15),
    (13, 81, 15), (15, 101, 17), (13, 97, 15), (15, 109, 13), (15, 93, 13),
];
const LOWER_BLAZE_POSITIONS: [(i32, i32, i32); 10] = [
    (17, 39, 15), (13, 63, 15), (15, 51, 17), (17, 55, 15), (15, 27, 13),
    (13, 31, 15), (15, 59, 13), (15, 35, 17), (13, 47, 15), (15, 43, 13),
];

/// The exact level number seen on every live-captured real nametag - not a guess.
const BLAZE_LEVEL_TEXT: &str = "Lv15";

/// Vertical offset for the health nametag above a Blaze's own position - a reasonable estimate
/// for a vanilla Blaze's real model height plus the usual small floating margin, not a captured
/// exact value (unlike `BLAZE_LEVEL_TEXT`) - adjust if it looks off in-game.
const NAMETAG_Y_OFFSET: f64 = 2.3;

/// How many ticks the reward chest's shaft waits between each single-block step - confirmed real
/// (per explicit request): "one block every 5 ticks".
pub const CHEST_STEP_TICKS: u64 = 5;
/// How far each correct in-order kill nudges the chest's target position along the shaft, before
/// clamping to `ChestShaftState::end_y` - confirmed real (per explicit request): "every time you
/// shoot a blaze, the thing goes down 5 blocks".
const CHEST_BLOCKS_PER_KILL: i32 = 5;

/// Lower Blaze's reward-chest shaft - confirmed real, captured directly: a vertical column of
/// Iron Bars at the room's dead center (local x15/z15, matching the blazes' own shared X/Z grid),
/// holding the actual reward chest at one Y position within it. Before any correct kill, the
/// chest sits at `LOWER_CHEST_START_Y` (69, just above the highest blaze at y63); each correct
/// kill moves it `CHEST_BLOCKS_PER_KILL` further down, clamped to `LOWER_CHEST_END_Y` (20) once
/// the puzzle's fully solved.
const LOWER_CHEST_X: i32 = 15;
const LOWER_CHEST_Z: i32 = 15;
const LOWER_CHEST_START_Y: i32 = 69;
const LOWER_CHEST_END_Y: i32 = 20;

/// Higher Blaze's own reward-chest shaft - same dead-center X/Z as Lower Blaze, same mechanic,
/// opposite direction: starts low (y70, below the lowest blaze at y77) and climbs to y119 once
/// solved.
const HIGHER_CHEST_X: i32 = 15;
const HIGHER_CHEST_Z: i32 = 15;
const HIGHER_CHEST_START_Y: i32 = 70;
const HIGHER_CHEST_END_Y: i32 = 119;

/// The real captured blessing skull texture - same constant (by value) as `boulder.rs`'s
/// `REWARD_BLESSING_TEXTURE`, `tic_tac_toe.rs`'s, `ice_path.rs`'s, and `teleport_maze.rs`'s -
/// each file keeps its own copy rather than importing a shared one. Per explicit request, the
/// chest that arrives at the bottom/top of the shaft is a blessing chest too, same as those.
const REWARD_BLESSING_TEXTURE: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZTkzZTIwNjg2MTc4NzJjNTQyZWNkYTFkMjdkZjRlY2U5MWM2OTk5MDdiZjMyN2M0ZGRiODUzMDk0MTJkMzkzOSJ9fX0=";

/// `(x, z, start_y, end_y)` for the given room variant's reward-chest shaft - see
/// `LOWER_CHEST_X`'s/`HIGHER_CHEST_X`'s doc comments.
fn chest_shaft_params(bottom: i32) -> Option<(i32, i32, i32, i32)> {
    match bottom {
        LOWER_BLAZE_BOTTOM => Some((LOWER_CHEST_X, LOWER_CHEST_Z, LOWER_CHEST_START_Y, LOWER_CHEST_END_Y)),
        HIGHER_BLAZE_BOTTOM => Some((HIGHER_CHEST_X, HIGHER_CHEST_Z, HIGHER_CHEST_START_Y, HIGHER_CHEST_END_Y)),
        _ => None,
    }
}

/// The reward chest's real block: a chest, sitting inside the Iron Bars shaft - see
/// `LOWER_CHEST_X`'s doc comment. "North" means the room's own canonical (pre-rotation) north,
/// not the world compass direction - `rotation` (the room's actual placed rotation) rotates it
/// to match, same as every other rotated block this room places.
fn chest_block(rotation: Direction) -> Blocks {
    let mut block = Blocks::Chest { direction: Direction::North };
    block.rotate(rotation);
    block
}

/// Animation state for one Blaze room's reward-chest shaft - see `LOWER_CHEST_X`'s doc comment
/// for the mechanic itself. `Some` only for the one room (if any) whose variant has a captured
/// shaft - set by `setup_chest_shaft`, ticked every room tick by `tick` below.
#[derive(Debug)]
pub struct ChestShaftState {
    x: i32,
    z: i32,
    /// `-1` if the chest moves down (Lower), `+1` if it moves up (Higher) - derived once at setup
    /// from comparing the shaft's start/end Y, rather than re-deriving it from `room_data.bottom`
    /// at every step.
    direction: i32,
    /// The shaft's far end - once `current_y` reaches this, the chest has fully arrived (solved).
    end_y: i32,
    /// Where the chest is actually placed in the world right now.
    current_y: i32,
    /// Where `current_y` is walking toward, one block at a time - advanced by
    /// `CHEST_BLOCKS_PER_KILL` (clamped to `end_y`) on each correct kill.
    target_y: i32,
    /// Ticks left before `current_y` takes its next single-block step toward `target_y`.
    next_step_in: u64,
    /// Set the first (and only) time `current_y` reaches `end_y` - guards the chest's one-shot
    /// transformation into a real, openable blessing chest so a later `tick` call (there isn't
    /// one right now, since the chest simply stops moving once arrived, but this is cheap
    /// insurance against that changing) can't fire it a second time.
    arrived: bool,
}

#[derive(Debug)]
pub struct BlazeState {
    /// Entity ids in the correct kill order (already sorted ascending/descending by HP, per room
    /// variant, at setup time) - position in this puzzle conveys nothing about order, only this
    /// does.
    correct_order: Vec<EntityId>,
    /// How many of `correct_order` have been correctly killed so far, in order.
    next_index: usize,
    /// Set the first time a kill happens out of order - once true, further kills are no longer
    /// order-checked (the puzzle's already failed; blazes still die normally so the room can
    /// still be cleared).
    failed: bool,
}

/// Sets up the Higher or Lower puzzle for `room` if it actually is one - generates this
/// dungeon's real HP values, shuffles them across the 10 fixed positions, spawns each Blaze with
/// its own health nametag, and records the resulting correct kill order. No-op for every other
/// room. Called once from `Room::load_into_world`.
pub fn setup(room: &mut Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Blaze" {
        return;
    }

    // Lower Blaze = entered from the top = kill highest HP first (descending) - see the module
    // doc comment for the three-way confirmation. Any bottom other than the two known real values
    // falls back to ascending (Higher Blaze's own behavior) rather than panicking - matches this
    // project's general preference for a safe default over a hard failure on unexpected data.
    let descending = room.room_data.bottom == LOWER_BLAZE_BOTTOM;
    let positions = if room.room_data.bottom == HIGHER_BLAZE_BOTTOM { &HIGHER_BLAZE_POSITIONS } else { &LOWER_BLAZE_POSITIONS };

    let mut rng = seeded_rng();
    let mut hps: Vec<u32> = Vec::with_capacity(10);
    let mut hp = rand::Rng::random_range(&mut rng, 0..=1000u32) + 1000;
    hps.push(hp);
    for _ in 1..10 {
        hp += rand::Rng::random_range(&mut rng, 0..=500u32) + 1;
        hps.push(hp);
    }
    rand::seq::SliceRandom::shuffle(hps.as_mut_slice(), &mut rng);

    let mut spawned: Vec<(EntityId, u32)> = Vec::with_capacity(10);
    for (&(x, y, z), &max_hp) in positions.iter().zip(hps.iter()) {
        let world_pos = room.get_world_block_pos(&BlockPos::new(x, y, z));
        let spawn_pos = DVec3::new(world_pos.x as f64 + 0.5, world_pos.y as f64, world_pos.z as f64 + 0.5);

        let Ok(entity_id) = world.spawn_entity(spawn_pos, EntityMetadata::new(EntityVariant::Blaze), BlazeMobImpl { room_index }) else { continue };

        // Plain, uncolored text, ONE space after the level bracket - matches Odin's real regex
        // `^\[Lv15] Blaze [\d,]+/([\d,]+)❤$` literally (re-read directly from `BlazeSolver.kt`).
        // A prior version had two spaces here, which was the actual reason Odin's solver never
        // matched this room - see the module doc comment for the full story.
        let nametag = format!("[{BLAZE_LEVEL_TEXT}] Blaze {hp}/{hp}\u{2764}", hp = format_with_commas(max_hp));
        let _ = spawn_following_nametag(world, entity_id, &nametag, NAMETAG_Y_OFFSET, EntityVariant::ArmorStand);

        spawned.push((entity_id, max_hp));
    }

    spawned.sort_by(|a, b| if descending { b.1.cmp(&a.1) } else { a.1.cmp(&b.1) });
    let correct_order = spawned.into_iter().map(|(id, _)| id).collect();

    room.blaze_state = Some(Rc::new(RefCell::new(BlazeState { correct_order, next_index: 0, failed: false })));

    setup_chest_shaft(room, world);
}

/// Places the reward chest at its starting position and records the shaft's animation state -
/// no-op if this room variant's shaft isn't captured yet (see `chest_shaft_params`). Called once
/// from `setup` above, after the blazes themselves are placed.
///
/// Deliberately does NOT pre-fill the rest of the shaft with Iron Bars - per explicit correction,
/// bars only ever trail directly above the chest's own current position (`tick` below lays one
/// down every time the chest steps away from a cell), never ahead of where it's actually been.
fn setup_chest_shaft(room: &mut Room, world: &mut World) {
    let Some((x, z, start_y, end_y)) = chest_shaft_params(room.room_data.bottom) else { return };

    let chest_pos = room.get_world_block_pos(&BlockPos::new(x, start_y, z));
    world.set_block_at(chest_block(room.rotation), chest_pos.x, chest_pos.y, chest_pos.z);

    let direction = if end_y < start_y { -1 } else { 1 };
    room.blaze_chest_state = Some(Rc::new(RefCell::new(ChestShaftState {
        x, z, direction, end_y,
        current_y: start_y,
        target_y: start_y,
        next_step_in: CHEST_STEP_TICKS,
        arrived: false,
    })));
}

/// One correct in-order kill just happened - nudges the chest shaft's target
/// `CHEST_BLOCKS_PER_KILL` further along, clamped to `end_y`. `tick` (below) does the actual
/// per-tick stepping toward whatever `target_y` ends up at. No-op if this room has no chest shaft
/// (Higher Blaze, for now - see `chest_shaft_params`).
fn advance_chest_shaft(room_index: usize, player: &mut Player) {
    let Some(state_rc) = player.server_mut().dungeon.rooms.get(room_index).and_then(|room| room.blaze_chest_state.clone()) else { return };
    let mut state = state_rc.borrow_mut();
    state.target_y = if state.direction < 0 {
        (state.target_y - CHEST_BLOCKS_PER_KILL).max(state.end_y)
    } else {
        (state.target_y + CHEST_BLOCKS_PER_KILL).min(state.end_y)
    };
}

/// Steps the reward chest one block closer to `target_y` every `CHEST_STEP_TICKS` ticks, if it's
/// not already there - the block it leaves becomes Iron Bars again, the block it moves into
/// (previously Iron Bars) becomes the chest. No-op for every room but the one with an active
/// `blaze_chest_state`. Called every room tick from `Room::tick`.
pub fn tick(room: &Room, world: &mut World) {
    if room.room_data.name != "Blaze" {
        return;
    }
    let Some(state_rc) = room.blaze_chest_state.clone() else { return };
    let mut state = state_rc.borrow_mut();

    if state.current_y == state.target_y {
        return;
    }
    if state.next_step_in > 0 {
        state.next_step_in -= 1;
        return;
    }

    let old_y = state.current_y;
    let new_y = old_y + state.direction;

    // Trailing bars only ever sit ABOVE the chest, never below, regardless of which way it's
    // moving - per explicit correction. Lower Blaze moves down, so the block it vacates is
    // already above the new one and becomes Iron Bars; Higher Blaze moves up, so the block it
    // vacates is below the new one and is cleared to Air instead of trailing a bar underneath it.
    let vacated_block = if state.direction < 0 { Blocks::IronBars } else { Blocks::Air };
    let old_pos = room.get_world_block_pos(&BlockPos::new(state.x, old_y, state.z));
    world.set_block_at(vacated_block, old_pos.x, old_pos.y, old_pos.z);

    let new_pos = room.get_world_block_pos(&BlockPos::new(state.x, new_y, state.z));
    world.set_block_at(chest_block(room.rotation), new_pos.x, new_pos.y, new_pos.z);

    state.current_y = new_y;
    state.next_step_in = CHEST_STEP_TICKS;

    // The chest just arrived at the far end of the shaft - turn it into a real, openable
    // blessing chest. Per explicit request, this is purely a reward: the puzzle's actual win
    // condition stays tied to killing the last blaze in order (see `handle_kill`'s `Solved`
    // branch above), NOT to opening this chest - so unlike Boulder/Teleport Maze/Ice Path's own
    // reward chests, this one deliberately never sets `puzzle_room_index`.
    if new_y == state.end_y && !state.arrived {
        state.arrived = true;
        spawn_blessing_chest(world, new_pos, room.rotation);
    }
}

/// Turns the reward chest into a real, interactable blessing chest once it arrives at the end of
/// its shaft - see `tick`'s own doc comment for why this never sets `puzzle_room_index`. `rotation`
/// is the room's own actual placed rotation - "north" means the room's canonical pre-rotation
/// north, not the world compass direction, same as `chest_block`.
fn spawn_blessing_chest(world: &mut World, pos: BlockPos, rotation: Direction) {
    let mut secret = crate::dungeon::room::secrets::DungeonSecret::new(
        crate::dungeon::room::secrets::SecretType::Chest { direction: Direction::North.rotate(rotation) },
        pos,
        0.0,
    );
    secret.blessing_texture = Some(REWARD_BLESSING_TEXTURE);
    secret.has_spawned = true;
    secret.counts_as_secret = false;
    let secret_rc = Rc::new(RefCell::new(secret));
    let secret_mut = secret_rc.borrow_mut();
    crate::dungeon::room::secrets::DungeonSecret::spawn_into_world(&secret_rc, secret_mut, world);
}

/// Formats a raw HP number the way the real captured nametags show it: full value, thousands
/// grouped with commas (`2062` -> `"2,062"`) - not `mob_type::format_health`'s abbreviated
/// `"2k"`/`"3.5M"` form used for every other dungeon mob's flavor nametag. Precise numbers matter
/// here specifically because reading them correctly is the entire puzzle.
fn format_with_commas(value: u32) -> String {
    let digits = value.to_string();
    let mut result = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

/// `EntityImpl` for a puzzle Blaze - stationary and passive (no `tick` behavior at all), any
/// weapon hit is an instant kill (matching this project's general no-real-damage-tracking combat
/// model - see the module doc comment), routed through `handle_kill` for the order check before
/// the normal death effects/despawn.
struct BlazeMobImpl {
    room_index: usize,
}

impl EntityImpl for BlazeMobImpl {
    fn tick(&mut self, _entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
        // Stationary and passive - see the module doc comment.
    }

    fn interact(&mut self, entity: &mut Entity, player: &mut Player, action: &EntityInteractionType) -> bool {
        if *action == EntityInteractionType::Attack {
            handle_kill(player, self.room_index, entity.id);
        }
        false
    }

    /// A Terminator shot hit this Blaze - same order-check/kill as a melee hit (`handle_kill`
    /// doesn't care which interaction type triggered it), except the projectile keeps flying
    /// afterward (`false`) instead of stopping here, per explicit request: the Terminator should
    /// pierce through one blaze to reach another standing behind it, not just kill the first one
    /// it touches.
    fn on_projectile_hit(&mut self, entity: &mut Entity, _velocity: DVec3, shooter_id: ClientId) -> bool {
        let entity_id = entity.id;
        let room_index = self.room_index;
        let world = entity.world_mut();
        if let Some(player) = world.players.get_mut(&shooter_id) {
            handle_kill(player, room_index, entity_id);
        }
        false
    }

    /// Explicit opt-in - see `EntityImpl::wants_projectile_hits`'s doc comment for why this can't
    /// just be inferred from visibility/entity-type.
    fn wants_projectile_hits(&self) -> bool {
        true
    }
}

/// What a single hit meant for puzzle progress - kept as an explicit enum rather than the
/// `Option<bool>` this used to be: that encoding had `Some(false)` mean *both* "correct kill,
/// just not the last one" (from `next_index == len` being false on every non-final correct kill)
/// *and* "wrong order, just failed" - the exact same value for two unrelated meanings dispatched
/// through the same match arm downstream, which is why PUZZLE FAIL fired on every single correct
/// kill except literally the 10th/last one. An enum can't collide like that.
enum KillOutcome {
    /// Correct kill, more of `correct_order` still remain.
    Progress,
    /// Correct kill, and it was the last one - puzzle solved.
    Solved,
    /// Wrong order - puzzle just failed on this hit.
    WrongOrder,
    /// Nothing to report: either the puzzle already failed earlier, or (see below) this is a
    /// redundant hit on a blaze already correctly processed - not a new event either way.
    Ignored,
}

/// One Blaze was hit - checks it against the correct kill order (no-op once already failed),
/// broadcasts PUZZLE FAIL/PUZZLE SOLVED at the appropriate moment, then applies the normal death
/// effects/despawn the same way every other dungeon mob kill does. Per explicit request, failing
/// (an out-of-order kill) despawns every other still-alive blaze too, nametags included - not just
/// the one that was actually hit.
fn handle_kill(player: &mut Player, room_index: usize, entity_id: EntityId) {
    let state_rc = player.server_mut().dungeon.rooms.get(room_index).and_then(|room| room.blaze_state.clone());

    if let Some(state_rc) = state_rc {
        let (outcome, remaining_alive) = {
            let mut state = state_rc.borrow_mut();
            if state.failed {
                (KillOutcome::Ignored, Vec::new())
            } else if state.correct_order[..state.next_index].contains(&entity_id) {
                // Already advanced past this exact entity - a redundant hit on a blaze that was
                // just correctly killed (e.g. the 1.8 client firing a second Attack packet for
                // the same swing before the despawn's Destroy Entities packet has reached it, so
                // it still looks clickable for one more packet) - not a new mistake to punish.
                (KillOutcome::Ignored, Vec::new())
            } else if state.correct_order.get(state.next_index) == Some(&entity_id) {
                state.next_index += 1;
                let outcome = if state.next_index == state.correct_order.len() { KillOutcome::Solved } else { KillOutcome::Progress };
                (outcome, Vec::new())
            } else {
                state.failed = true;
                // Every blaze from here on wasn't correctly killed yet - the one actually hit is
                // handled by the normal `combat::kill_mob` call below, so it's excluded here to
                // avoid despawning it twice.
                let remaining: Vec<EntityId> = state.correct_order[state.next_index..]
                    .iter()
                    .copied()
                    .filter(|&id| id != entity_id)
                    .collect();
                (KillOutcome::WrongOrder, remaining)
            }
        };

        match outcome {
            KillOutcome::Progress | KillOutcome::Solved => {
                // Real captured feedback cue for a correct kill - always heard by the killer
                // regardless of distance (per explicit request: "it shouldn't come from the
                // blaze it should come from me... I should always be able to hear the sound"),
                // so both sounds are positioned at the player's own position rather than the
                // blaze's, and sent only to them, not broadcast to the whole party.
                let pos = player.position;
                player.write_packet(&SoundEffect { sound: Sounds::NotePling.id(), pos_x: pos.x, pos_y: pos.y, pos_z: pos.z, volume: 8.0, pitch: 4.048 });
                player.write_packet(&SoundEffect { sound: Sounds::Orb.id(), pos_x: pos.x, pos_y: pos.y, pos_z: pos.z, volume: 1.0, pitch: 1.492 });

                // Every correct in-order kill nudges the reward-chest shaft along, regardless of
                // whether this was the final (Solved) one - per explicit request.
                advance_chest_shaft(room_index, player);

                if matches!(outcome, KillOutcome::Solved) {
                    let server = player.server_mut();
                    if let Some(room) = server.dungeon.rooms.get_mut(room_index) {
                        room.puzzle_completed = true;
                    }
                    let message = "\u{a7}a\u{a7}lPUZZLE SOLVED! \u{a7}7The Higher or Lower puzzle has been completed!".to_string();
                    for other_player in server.world.players.values_mut() {
                        other_player.send_message(&message);
                    }
                    server.dungeon.update_map_for_room(room_index);
                }
            }
            KillOutcome::WrongOrder => {
                let username = player.profile.username.clone();
                let message = format!("\u{a7}c\u{a7}lPUZZLE FAIL! \u{a7}a{username} \u{a7}efailed the Higher Or Lower puzzle!");
                for other_player in player.server_mut().world.players.values_mut() {
                    other_player.send_message(&message);
                }

                // Quiet despawn (no per-entity death sound/status) - these weren't actually hit,
                // they're clearing away as a consequence of the puzzle failing, not dying.
                // `despawn_entity` already takes each one's own following-nametag along with it.
                let world = player.world_mut();
                for other_id in remaining_alive {
                    world.despawn_entity(other_id);
                }

                // Every other puzzle's fail path does this triplet (score penalty, map
                // checkmark, tab-list ✖ instead of ✔) - this one never had it at all, so a
                // failed Higher or Lower puzzle previously left the room looking un-entered on
                // the map and untouched in the dungeon score.
                player.server_mut().dungeon.record_puzzle_failed();
                let server = player.server_mut();
                if let Some(room) = server.dungeon.rooms.get_mut(room_index) {
                    room.puzzle_completed = true;
                    room.puzzle_failed = true;
                }
                server.dungeon.update_map_for_room(room_index);
            }
            KillOutcome::Ignored => {}
        }
    }

    combat::kill_mob(player.world_mut(), entity_id);
}

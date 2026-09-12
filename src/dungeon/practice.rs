//! Practice mode: build a single, isolated room (picked by name, not by dungeon layout string)
//! into the same fixed dungeon grid a real run uses, spawn into it from a chosen (or random)
//! door, and instantly reset it back to that same spot via `/rs`.
//!
//! Reuses existing machinery end to end rather than re-implementing any of it:
//! - `Room::new` + `Room::load_into_world` already rebuild a room's blocks/secrets/crypts/
//!   superboomwalls/fallingblocks/levers from scratch from the room's immutable `RoomData`
//!   blueprint - calling them again on a fresh `Room` *is* "reset the room".
//! - `main::populate_dungeon_world` already places rooms/doors/locked chests generically over
//!   whatever `dungeon.rooms`/`dungeon.doors` contains - it doesn't assume a full 6x6 dungeon,
//!   so it works unmodified for a 1-room practice dungeon.
//! - `dungeon_switch`'s teardown/wipe/resync helpers already implement the fix for "ghost
//!   blocks" (the vanilla client/server block desync artifact): a full chunk resend is exactly
//!   the normal unload/reload fix, so practice mode reuses those helpers verbatim instead of
//!   tracking and undoing individual block changes.
//!
//! The practice room always sits at a fixed spot (grid cell `(1,1)`) within the *same*
//! `DUNGEON_ORIGIN` grid a real dungeon uses, and every reset/reload wipes+resyncs the whole
//! grid's chunk range (`dungeon_switch::full_dungeon_chunk_bounds`) rather than a
//! shape-dependent bounding box - nothing else ever occupies this grid in a practice-mode
//! server, so there's no need for separate per-shape wipe-bounds math.

use crate::dungeon::door::{Door, DoorType};
use crate::dungeon::dungeon::{Dungeon, DUNGEON_ORIGIN};
use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::room::room::{Room, RoomSegment};
use crate::dungeon::room::room_data::{RoomData, RoomShape, RoomType};
use crate::net::protocol::play::clientbound::DestroyEntites;
use crate::net::var_int::VarInt;
use crate::server::block::block_parameter::Axis;
use crate::server::dungeon_switch::{discard_chunk_packet_noise, full_dungeon_chunk_bounds, resync_chunk_range, wipe_chunk_range};
use crate::server::entity::entity::EntityId;
use crate::server::redstone::RedstoneSystem;
use crate::server::server::Server;
use crate::server::utils::direction::Direction;
use crate::server::utils::dvec3::DVec3;
use rand::seq::IndexedRandom;

/// Where the practice room's segments start within the normal 6x6 dungeon grid - leaves margin
/// on every side for shapes up to 4 cells long (`OneByFour`) or 2x2 (`TwoByTwo`).
const PRACTICE_GRID_ORIGIN: (usize, usize) = (1, 1);

#[derive(Debug, Clone)]
pub struct PracticeDoorSpawn {
    pub label: &'static str,
    pub inside_pos: DVec3,
    pub inside_yaw: f32,
}

fn supported_shape(shape: &RoomShape) -> bool {
    !matches!(shape, RoomShape::FourByFour | RoomShape::Empty)
}

/// Room names available to `/practice`, for tab-completion and validation - excludes shapes
/// practice mode can't build a room for (currently just the boss room's `FourByFour`).
///
/// Chat commands split on whitespace, so a room whose real name has a space in it (e.g. "Quartz
/// Knight") can never be typed as a single argument - these are listed (and matched, see
/// `find_room_data`) with spaces replaced by underscores instead (`quartz_knight`).
///
/// "Blaze" is excluded from this generic listing and replaced with `higher_blaze`/`lower_blaze`
/// instead (see `find_room_data`'s own doc comment for why) - the plain `blaze` name would be
/// genuinely ambiguous (this project's own data model gives both physical rooms the identical
/// `room_data.name` "Blaze", only distinguishable by `bottom`), so surfacing it unqualified would
/// silently pick whichever of the two happens to iterate first.
pub fn supported_room_names(server: &Server) -> Vec<String> {
    let mut names: Vec<String> = server.room_data_storage.values()
        .filter(|data| data.room_type != RoomType::Boss && supported_shape(&data.shape) && data.name != "Blaze")
        .map(|data| data.name.replace(' ', "_"))
        .collect();
    names.push("higher_blaze".to_string());
    names.push("lower_blaze".to_string());
    // `water_board_0`..`water_board_3` - see `find_room_data`'s own special case: each picks one
    // specific real captured pattern (`RoomData::id` `water_board_pattern_N`) out of the 5 real
    // "Water Board" room variants, deliberately, instead of plain `water_board`'s arbitrary pick.
    for pattern in 0..4 {
        names.push(format!("water_board_{pattern}"));
    }
    names.sort();
    names.dedup();
    names
}

/// `higher_blaze`/`lower_blaze` are special-cased ahead of the generic name match - both physical
/// Blaze rooms share the identical `room_data.name` "Blaze" (this project's own data model doesn't
/// split them into two named rooms the way Odin does - see `blaze.rs`'s own module doc comment),
/// so `bottom` (15 vs 65, `blaze::HIGHER_BLAZE_BOTTOM`/`LOWER_BLAZE_BOTTOM`) is the only field that
/// actually tells them apart. Without this, `/practice blaze` could only ever reach whichever of
/// the two happened to iterate first out of `room_data_storage` - no way to deliberately pick the
/// other one to test.
fn find_room_data(server: &Server, room_name: &str) -> Option<RoomData> {
    let lower = room_name.to_ascii_lowercase();
    if lower == "higher_blaze" || lower == "lower_blaze" {
        let want_bottom = if lower == "higher_blaze" { crate::dungeon::room::blaze::HIGHER_BLAZE_BOTTOM } else { crate::dungeon::room::blaze::LOWER_BLAZE_BOTTOM };
        return server.room_data_storage.values()
            .find(|data| data.name == "Blaze" && data.bottom == want_bottom)
            .cloned();
    }

    // `water_board_0`..`water_board_3` - Water Board's real maze pattern is purely internal
    // variety (exactly ONE real `room_data_storage` entry, see `waterboard::pick_pattern`'s own
    // doc comment for why - NOT 4 separate storage entries, unlike Blaze's real Higher/Lower
    // split just above), normally rolled fresh at real dungeon-generation time. This deterministic
    // variant (`waterboard::apply_pattern`, the same swap-and-tag logic `pick_pattern` itself
    // uses, just caller-picked instead of rolled) lets `/practice` deliberately test one specific
    // real pattern instead of whatever `pick_pattern` would have randomly chosen.
    if let Some(pattern) = lower.strip_prefix("water_board_").and_then(|rest| rest.parse::<u8>().ok()) {
        if pattern < 4 {
            let mut data = server.room_data_storage.values()
                .find(|data| data.name == "Water Board")?
                .clone();
            crate::dungeon::room::waterboard::apply_pattern(&mut data, pattern);
            return Some(data);
        }
    }

    let normalized = room_name.replace('_', " ");
    server.room_data_storage.values()
        .find(|data| data.room_type != RoomType::Boss
            && supported_shape(&data.shape)
            && data.name.eq_ignore_ascii_case(&normalized))
        .cloned()
}

/// Local (unrotated) segment layout for each supported shape. Any deterministic arrangement in
/// the right shape family produces a correctly self-consistent rotation via
/// `Room::get_rotation_from_segments` - which arrangement doesn't matter for correctness, only
/// that segments of the same room are actually adjacent.
fn shape_segments(shape: &RoomShape) -> Option<Vec<(usize, usize)>> {
    use RoomShape::*;
    Some(match shape {
        OneByOne | OneByOneEnd | OneByOneCross | OneByOneStraight | OneByOneBend | OneByOneTriple => vec![(0, 0)],
        OneByTwo => vec![(0, 0), (1, 0)],
        OneByThree => vec![(0, 0), (1, 0), (2, 0)],
        OneByFour => vec![(0, 0), (1, 0), (2, 0), (3, 0)],
        L => vec![(0, 0), (1, 0), (0, 1)],
        TwoByTwo => vec![(0, 0), (1, 0), (0, 1), (1, 1)],
        FourByFour | Empty => return None,
    })
}

/// Which (local segment, outward compass side) pairs get a door.
///
/// For the `OneByOne*` shapes this is the *exact* reverse of the door bit-pattern
/// `Room::get_1x1_shape_and_type` already uses to recognize these shapes at canonical (North)
/// rotation - not a guess, just read backwards out of existing code. Fairy's `OneByOne` gets all
/// four sides since its real door count varies 1-4 depending on layout, so its block design must
/// already support any side.
///
/// Multi-segment shapes have no such ground truth (the engine never records which of a
/// multi-cell room's outer edges are real doors), so these use a documented, generic geometric
/// default: one door at each open end of a straight run, one at the open end of each arm of an
/// L, and one centered on each outer side of a 2x2.
fn shape_doors(shape: &RoomShape, room_type: RoomType) -> Vec<((usize, usize), Direction)> {
    use Direction::*;
    use RoomShape::*;
    match shape {
        OneByOne if room_type == RoomType::Fairy =>
            vec![((0, 0), North), ((0, 0), East), ((0, 0), South), ((0, 0), West)],
        OneByOneCross => vec![((0, 0), North), ((0, 0), East), ((0, 0), South), ((0, 0), West)],
        OneByOneEnd => vec![((0, 0), North)],
        OneByOneStraight => vec![((0, 0), East), ((0, 0), West)],
        OneByOneBend => vec![((0, 0), South), ((0, 0), West)],
        OneByOneTriple => vec![((0, 0), North), ((0, 0), South), ((0, 0), West)],
        // Any other room_type with a raw OneByOne shape shouldn't normally occur - fall back to
        // every side rather than leaving the room door-less.
        OneByOne => vec![((0, 0), North), ((0, 0), East), ((0, 0), South), ((0, 0), West)],
        OneByTwo => vec![((0, 0), West), ((1, 0), East)],
        OneByThree => vec![((0, 0), West), ((2, 0), East)],
        OneByFour => vec![((0, 0), West), ((3, 0), East)],
        L => vec![((1, 0), East), ((0, 1), South)],
        TwoByTwo => vec![((0, 0), North), ((1, 0), East), ((1, 1), South), ((0, 1), West)],
        FourByFour | Empty => Vec::new(),
    }
}

fn direction_label(dir: Direction) -> &'static str {
    match dir {
        Direction::North => "north",
        Direction::East => "east",
        Direction::South => "south",
        Direction::West => "west",
        _ => "north",
    }
}

/// Yaw that faces *into* the room from a door on the given outward side - i.e. facing the
/// opposite compass direction. Real (vanilla) yaw is South=0/West=90/North=180/East=270, so
/// facing into the room from e.g. a north-side door (facing South) is yaw 0.
fn inward_yaw(outward: Direction) -> f32 {
    match outward {
        Direction::North => 0.0,
        Direction::East => 90.0,
        Direction::South => 180.0,
        Direction::West => 270.0,
        _ => 0.0,
    }
}

/// Builds a fresh, isolated single-room `Dungeon` for `room_data`, plus every door it ended up
/// with (label + a spawn point just inside it, facing into the room).
pub fn build_practice_dungeon(room_data: RoomData) -> anyhow::Result<(Dungeon, Vec<PracticeDoorSpawn>)> {
    let Some(local_segments) = shape_segments(&room_data.shape) else {
        anyhow::bail!("Room shape {:?} isn't supported in practice mode", room_data.shape);
    };

    let (origin_x, origin_z) = PRACTICE_GRID_ORIGIN;
    let segments: Vec<RoomSegment> = local_segments.iter()
        .map(|(lx, lz)| RoomSegment { x: lx + origin_x, z: lz + origin_z, neighbours: [const { None }; 4] })
        .collect();

    let mut doors: Vec<Door> = Vec::new();
    let mut door_spawns: Vec<PracticeDoorSpawn> = Vec::new();

    for ((lx, lz), dir) in shape_doors(&room_data.shape, room_data.room_type) {
        let grid_x = lx + origin_x;
        let grid_z = lz + origin_z;
        let center_x = grid_x as i32 * 32 + 15 + DUNGEON_ORIGIN.0;
        let center_z = grid_z as i32 * 32 + 15 + DUNGEON_ORIGIN.1;
        let (ox, _, oz) = dir.get_offset();

        let door_x = center_x + ox * 16;
        let door_z = center_z + oz * 16;
        // Same parity rule `Dungeon::from_str` uses to assign an Axis to each of the fixed
        // `DOOR_POSITIONS` - it only depends on the door's world X coordinate, so it applies
        // identically to a door position built here.
        let axis = if ((door_x - DUNGEON_ORIGIN.0) / 16) % 2 == 0 { Axis::Z } else { Axis::X };

        doors.push(Door { x: door_x, z: door_z, direction: axis, door_type: DoorType::NORMAL, key_granted: false, opened: false });

        door_spawns.push(PracticeDoorSpawn {
            label: direction_label(dir),
            inside_pos: DVec3::new((door_x - ox * 2) as f64 + 0.5, 69.0, (door_z - oz * 2) as f64 + 0.5),
            inside_yaw: inward_yaw(dir),
        });
    }

    let room = Room::new(segments, &doors, room_data);
    let dungeon = Dungeon::from_layout(doors, vec![room])?;

    Ok((dungeon, door_spawns))
}

/// Tears down whatever currently occupies the dungeon grid and replaces it with
/// `new_dungeon`, then resyncs every connected player - the same teardown-then-resend pattern
/// `dungeon_switch::switch_dungeon` uses for full dungeons, including its fix for "ghost
/// blocks": a full chunk resend is exactly the vanilla unload/reload fix for a client/server
/// block desync, so reusing it here needs no new mechanism.
fn apply_practice_dungeon(server: &mut Server, mut new_dungeon: Dungeon) -> anyhow::Result<()> {
    new_dungeon.server = server as *mut Server;
    new_dungeon.practice_room = true;
    // Skip the normal countdown/Mort-flavor-text lobby entirely - practice rooms start "live".
    new_dungeon.state = DungeonState::Started { current_ticks: 0 };

    server.tasks.clear();

    let old_entity_ids: Vec<EntityId> = server.world.entities.keys().copied().collect();
    server.world.entities.clear();
    server.world.entity_equipment.clear();
    server.world.entity_combat_state.clear();
    server.world.entity_ai_suspended.clear();
    server.world.entity_attack_cooldown.clear();
    server.world.entity_current_target.clear();
    server.world.entity_following_nametag.clear();
    server.world.entity_mob_ai.clear();
    server.world.entity_starred_mob_room.clear();
    server.world.entity_crypt_room.clear();
    server.world.entities_for_removal.clear();
    server.world.interactable_blocks.clear();
    server.world.redstone_system = RedstoneSystem::new();
    server.world.tactical_insertions.clear();
    server.world.scheduled_fixed_sounds.clear();

    server.dungeon.locked_chests.clear();
    server.dungeon.lever_to_chests.clear();

    let (min_x, max_x, min_z, max_z) = full_dungeon_chunk_bounds();
    wipe_chunk_range(&mut server.world, min_x, max_x, min_z, max_z);

    server.dungeon = new_dungeon;

    crate::populate_dungeon_world(server)?;

    for (_, player) in server.world.players.iter_mut() {
        player.write_packet(&DestroyEntites {
            entities: old_entity_ids.iter().map(|id| VarInt(*id)).collect(),
        });

        let world = player.world_mut();
        resync_chunk_range(world, player, min_x, max_x, min_z, max_z);
        player.flush_packets();
    }

    discard_chunk_packet_noise(&mut server.world, min_x, max_x, min_z, max_z);

    Ok(())
}

fn teleport_player(player: &mut crate::server::player::player::Player, pos: DVec3, yaw: f32, pitch: f32) {
    // Yaw/pitch aren't part of the position-reconciliation race `server_teleport` guards
    // against (see `PendingTeleport`'s doc comment) - flags 0 means fully absolute look too, so
    // set them directly here the same way this always has.
    player.yaw = yaw;
    player.last_yaw = yaw;
    player.pitch = pitch;
    player.last_pitch = pitch;
    player.practice_last_spawn = Some((pos, yaw, pitch));

    // `server_teleport` leaves `last_position` alone (see its own doc comment), so the normal
    // per-tick "you moved, resync newly-visible chunks" diff already fires correctly for this
    // jump on its own - that covers any chunk the client has genuinely never been told about.
    // `flush_all_pending_chunk_sync` below is for a separate, narrower race: chunks that WERE
    // already queued as part of the original join snapshot (see `server::sync_player_view`'s
    // staggering) but might not have finished sending yet if `/practice` fires soon enough after
    // joining - that queue is independent of the per-tick diff mechanism, so it needs its own
    // explicit flush. See `server::flush_all_pending_chunk_sync`'s doc comment for the full story
    // (this is what was silently breaking Odin's one-shot room-identification for
    // `/practice`-teleported rooms, Blaze most of all).
    player.server_teleport(pos, yaw, pitch, 0);
    crate::server::server::flush_all_pending_chunk_sync(player);

    player.flush_packets();
}

/// `/practice <room> <door> <secrets>` - loads a fresh, isolated copy of the named room and
/// spawns every connected player just inside the requested door (or a random valid one if
/// `door_choice` is `"random"`), and arms the secret-route timer for `target_secrets` (see
/// `Dungeon::tick`). Secrets spawn exactly like a normal run - proximity/room-entry gated, not
/// force-spawned.
pub fn load_practice_room(server: &mut Server, room_name: &str, door_choice: &str, target_secrets: u8) -> anyhow::Result<()> {
    let Some(room_data) = find_room_data(server, room_name) else {
        anyhow::bail!("Unknown or unsupported practice room: '{}'", room_name);
    };
    let display_name = room_data.name.clone();
    let secrets_total = room_data.secrets;

    if target_secrets == 0 {
        anyhow::bail!("Secret count must be at least 1.");
    }
    if secrets_total > 0 && target_secrets > secrets_total {
        anyhow::bail!("{} only has {} secret(s).", display_name, secrets_total);
    }

    let (dungeon, door_spawns) = build_practice_dungeon(room_data)?;

    let chosen = if door_choice.eq_ignore_ascii_case("random") {
        door_spawns.choose(&mut rand::rng()).cloned()
    } else {
        door_spawns.iter().find(|d| d.label.eq_ignore_ascii_case(door_choice)).cloned()
    };

    let Some(chosen) = chosen else {
        let valid = door_spawns.iter().map(|d| d.label).collect::<Vec<_>>().join(", ");
        anyhow::bail!("{} has no {} door. Valid doors: {}", display_name, door_choice, valid);
    };

    let door_letter = chosen.label.chars().next().unwrap_or('?').to_ascii_uppercase();
    let route_prefix = format!("{} {}{}/{}", display_name, door_letter, target_secrets, secrets_total);

    apply_practice_dungeon(server, dungeon)?;
    server.dungeon.practice_target_secrets = Some(target_secrets);
    server.dungeon.practice_route_start_tick = None;
    server.dungeon.practice_route_finished = false;
    server.dungeon.practice_route_finish_seconds = None;
    server.dungeon.practice_route_prefix = Some(route_prefix);
    server.dungeon.practice_last_found_secrets = 0;
    server.dungeon.practice_route_flash_ticks = 0;
    server.dungeon.practice_timer_text = Some("&60.00s".to_string());

    for (_, player) in server.world.players.iter_mut() {
        teleport_player(player, chosen.inside_pos, chosen.inside_yaw, 0.0);
    }

    Ok(())
}

/// `/rs` - rebuilds the current practice room from scratch (fresh blocks, secrets, crypts,
/// superboomwalls, falling blocks, levers, locked chests; no leftover "ghost" blocks since every
/// connected player gets a full chunk resend) and returns each player to wherever they last
/// spawned into this room. The secret-route target from the last `/practice` call carries over
/// unchanged (only the timer and its finished-latch reset) - `/rs` is for re-attempting the same
/// route, not picking a new one.
pub fn restart_practice_room(server: &mut Server) -> anyhow::Result<()> {
    if !server.dungeon.practice_room || server.dungeon.rooms.is_empty() {
        anyhow::bail!("No practice room is currently loaded. Use /practice <room> <door> <secrets> first.");
    }

    let room_data = server.dungeon.rooms[0].room_data.clone();
    let target_secrets = server.dungeon.practice_target_secrets;
    let route_prefix = server.dungeon.practice_route_prefix.clone();
    let (dungeon, door_spawns) = build_practice_dungeon(room_data)?;
    let fallback = door_spawns.first()
        .map(|d| (d.inside_pos, d.inside_yaw, 0.0f32))
        .unwrap_or((DVec3::new(0.5, 69.0, 0.5), 0.0, 0.0));

    apply_practice_dungeon(server, dungeon)?;
    server.dungeon.practice_target_secrets = target_secrets;
    server.dungeon.practice_route_prefix = route_prefix;
    server.dungeon.practice_route_start_tick = None;
    server.dungeon.practice_route_finished = false;
    server.dungeon.practice_route_finish_seconds = None;
    server.dungeon.practice_last_found_secrets = 0;
    server.dungeon.practice_route_flash_ticks = 0;
    server.dungeon.practice_timer_text = Some("&60.00s".to_string());

    for (_, player) in server.world.players.iter_mut() {
        let (pos, yaw, pitch) = player.practice_last_spawn.unwrap_or(fallback);
        teleport_player(player, pos, yaw, pitch);
    }

    Ok(())
}

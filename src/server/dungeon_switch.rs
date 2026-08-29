//! Instantly leaves whatever dungeon is currently active (if any) and starts a brand new,
//! fully independent one - the server-side half of the CNC menu's "Click to play anyway!"
//! action (see `container_ui.rs`).
//!
//! The whole operation is synchronous: nothing here awaits or yields back to the network/tick
//! loop, so from every connected player's perspective the transition is a single atomic burst
//! of packets (destroy old entities, fresh chunk data, fresh entity spawns, teleport) - there is
//! never a tick where a client could move around or interact with a half-torn-down or
//! half-built dungeon.
//!
//! Ordering is deliberate: every server-side mutation (clearing old state, wiping the dungeon's
//! chunk region, building the new `Dungeon`, repopulating it) happens *before* any packet is
//! sent to a player. Only once the new dungeon fully exists do we start writing to player
//! connections.

use crate::dungeon::dungeon::{Dungeon, DUNGEON_ORIGIN};
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{CloseWindow, DestroyEntites, Maps, PlayerListItem, PositionLook, Teams};
use crate::net::var_int::VarInt;
use crate::server::chunk::chunk::Chunk;
use crate::server::entity::entity::EntityId;
use crate::server::entity::spawn_equipped::send_equipment_packets;
use crate::server::player::container_ui::UI;
use crate::server::player::dungeon_stats::DungeonPlayerStats;
use crate::server::player::player::{GameProfile, Player};
use crate::server::player::scoreboard::REMOVE_TEAM;
use crate::server::redstone::RedstoneSystem;
use crate::server::server::Server;
use crate::server::utils::player_list::player_profile::{GameType, PlayerData};
use crate::server::utils::sized_string::SizedString;
use crate::server::world::{write_entity_spawn, World};
use crate::utils::seeded_rng::SeededRng;
use rand::seq::IndexedRandom;
use std::collections::HashMap;

/// Removes the client's tab-list entry and hidden-nameplate ("npc_hide") team for the
/// *previous* Mort before the new one spawns.
///
/// Mort reuses the exact same UUID and team name in every dungeon (`crate::MORT_UUID_STR`,
/// `"npc_hide"` in `populate_dungeon_world`), since he's meant to look like the same NPC each
/// time. Without this cleanup, a client that already has tab-list/team state for that UUID/name
/// from the dungeon we just left sees a duplicate `ADD_PLAYER`/`CREATE_TEAM` for state it
/// already has when the new Mort spawns - which is exactly the kind of malformed sequence that
/// can corrupt a client's packet handling for the rest of that burst (chunk data, other entity
/// spawns included), not just Mort himself.
fn remove_stale_mort_client_state(player: &mut Player) {
    let mort_uuid = uuid::Uuid::parse_str(crate::MORT_UUID_STR).unwrap();
    let removal_profile = GameProfile {
        uuid: mort_uuid,
        username: String::new(),
        properties: HashMap::new(),
    };
    let removal_data = PlayerData {
        ping: 0,
        game_mode: GameType::Survival,
        profile: removal_profile,
        display_name: None,
    };
    player.write_packet(&PlayerListItem {
        action: VarInt(4), // REMOVE_PLAYER
        players: vec![&removal_data],
    });
    player.write_packet(&Teams {
        name: SizedString::truncated("npc_hide"),
        display_name: SizedString::truncated(""),
        prefix: SizedString::truncated(""),
        suffix: SizedString::truncated(""),
        name_tag_visibility: SizedString::truncated("never"),
        color: 0,
        players: vec![],
        action: REMOVE_TEAM,
        friendly_flags: 0,
    });
}

/// How many rooms across the (fixed) dungeon grid is, and how many blocks each room cell spans
/// - mirrors the layout math in `dungeon.rs` (`DUNGEON_ORIGIN`, `room_grid: [Option<usize>; 36]`).
const DUNGEON_GRID_CELLS: i32 = 6;
const DUNGEON_CELL_SIZE: i32 = 32;
/// Extra padding (in blocks) around the room grid's exact bounds when wiping chunks, so door
/// structures and anything else that pokes slightly past a room's own footprint can't leave a
/// stray block behind.
const CLEAR_MARGIN_BLOCKS: i32 = 32;

/// The chunk range covering the entire fixed 6x6 room grid (+ margin). Shared by
/// `switch_dungeon` (which always uses the whole grid) and practice mode (`dungeon::practice`),
/// which only ever has a single room living somewhere inside this same grid - reusing the whole
/// grid's bounds there too avoids needing separate, shape-dependent bounding-box math just to
/// wipe/resync a practice room.
pub(crate) fn full_dungeon_chunk_bounds() -> (i32, i32, i32, i32) {
    let min_chunk_x = (DUNGEON_ORIGIN.0 - CLEAR_MARGIN_BLOCKS) >> 4;
    let min_chunk_z = (DUNGEON_ORIGIN.1 - CLEAR_MARGIN_BLOCKS) >> 4;
    let max_chunk_x = (DUNGEON_ORIGIN.0 + DUNGEON_GRID_CELLS * DUNGEON_CELL_SIZE + CLEAR_MARGIN_BLOCKS) >> 4;
    let max_chunk_z = (DUNGEON_ORIGIN.1 + DUNGEON_GRID_CELLS * DUNGEON_CELL_SIZE + CLEAR_MARGIN_BLOCKS) >> 4;
    (min_chunk_x, max_chunk_x, min_chunk_z, max_chunk_z)
}

/// Wipes every chunk in the given (inclusive) chunk range back to a truly empty chunk - not a
/// block-by-block Air fill, which would be both slow and would spam a BlockChange packet per
/// block.
pub(crate) fn wipe_chunk_range(world: &mut World, min_x: i32, max_x: i32, min_z: i32, max_z: i32) {
    for chunk_x in min_x..=max_x {
        for chunk_z in min_z..=max_z {
            if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                *chunk = Chunk::new();
            }
        }
    }
}

/// Discards the block/entity-spawn packet noise generated while (re)building a dungeon into the
/// given chunk range - already delivered explicitly via `resync_chunk_range`, so leaving it
/// would just get redundantly re-sent on the next normal tick.
pub(crate) fn discard_chunk_packet_noise(world: &mut World, min_x: i32, max_x: i32, min_z: i32, max_z: i32) {
    for chunk_x in min_x..=max_x {
        for chunk_z in min_z..=max_z {
            if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                chunk.packet_buffer = PacketBuffer::new();
            }
        }
    }
}

/// Sends fresh chunk data + entity/equipment spawns for every chunk in the given (inclusive)
/// range to `player`, regardless of their current position or view distance.
///
/// Deliberately NOT `server::sync_player_view` (which windows around the player's *current*
/// position via view distance): the region `switch_dungeon` just wiped and rebuilt can be wider
/// than a view-distance window from the new spawn point covers, and any chunk left out of the
/// resync would be blank/rebuilt server-side but never actually told to the client - showing as
/// a hole straight through to the void even though the new dungeon's geometry is really there.
/// Using the exact same bounds that were wiped guarantees full coverage with no gap.
pub(crate) fn resync_chunk_range(world: &mut World, player: &mut Player, min_x: i32, max_x: i32, min_z: i32, max_z: i32) {
    for chunk_x in min_x..=max_x {
        for chunk_z in min_z..=max_z {
            let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) else { continue };
            player.write_packet(&chunk.get_chunk_data(chunk_x, chunk_z, true));

            for entity_id in chunk.entities.iter_mut() {
                let Some((entity, entity_impl)) = world.entities.get_mut(&entity_id) else { continue };
                write_entity_spawn(entity, entity_impl.as_mut(), &mut player.packet_buffer);
                if let Some(equipment) = world.entity_equipment.get(&*entity_id) {
                    send_equipment_packets(&mut player.packet_buffer, *entity_id, equipment);
                }
            }
        }
    }
}

/// Leaves the current dungeon (if any) and starts a brand new one, independent of whatever was
/// running before. Safe to call whether or not a dungeon is currently in progress - there's no
/// special-casing for "already inside a dungeon" vs not, since a full teardown-then-rebuild
/// handles both uniformly.
///
/// Every connected player lands in the new dungeon's entrance ("green") room - same spot a
/// brand-new player joining would spawn at (see `populate_dungeon_world`'s `set_spawn_point`
/// call for the Entrance room), not wherever they happened to be standing in the old one.
pub fn switch_dungeon(server: &mut Server) -> anyhow::Result<()> {
    // --- 1. Pick the new layout up front (pure data, doesn't touch world/dungeon state yet) ---
    let dungeon_strings = include_str!("../dungeon_storage/dungeons.txt")
        .split("\n")
        .collect::<Vec<&str>>();
    let mut picker_rng = rand::rng();
    let dungeon_str = dungeon_strings.choose(&mut picker_rng)
        .copied()
        .unwrap_or("080809010400100211121300101415161304171418161300191403161304191905160600919999113099910991099909090099999919990929999999099999999009");

    let mut new_dungeon = Dungeon::from_str(dungeon_str, &server.room_data_storage)?;
    new_dungeon.server = server as *mut Server;

    // --- 2. Tear down every trace of the old run ---

    // Timers/delayed actions: anything still pending (door-open animations, falling blocks,
    // lever unlock delays, ...) must never fire against the new dungeon.
    server.tasks.clear();

    // Every entity that existed belonged to the old run (mobs, Mort, nametags, dropped items,
    // projectiles) - collect their IDs first so we can tell clients to drop them, then drop all
    // server-side state in one go.
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

    // Doors, chests, levers, crypt/superboomwall/fallingblock triggers, etc - all keyed by a
    // fixed world BlockPos that the new dungeon will reuse, so a stale entry here could
    // misfire against a completely unrelated block in the new layout.
    server.world.interactable_blocks.clear();

    // Dead code today (see `RedstoneSystem`'s own doc comments) but keyed by BlockPos the same
    // way - reset for the same reason, in case it's ever wired up.
    server.world.redstone_system = RedstoneSystem::new();

    server.world.tactical_insertions.clear();
    server.world.scheduled_fixed_sounds.clear();

    // Wipe every block in (and slightly around) the dungeon's footprint back to a truly empty
    // chunk. `populate_dungeon_world` repopulates these chunks next.
    let (min_chunk_x, max_chunk_x, min_chunk_z, max_chunk_z) = full_dungeon_chunk_bounds();
    wipe_chunk_range(&mut server.world, min_chunk_x, max_chunk_x, min_chunk_z, max_chunk_z);

    // Replacing `server.dungeon` drops every last bit of the old run's own state in one move -
    // rooms (and each room's secrets/crypts/superboomwalls/fallingblocks/levers/mushroom
    // sets), doors, room_grid, DungeonState, the map, score, locked chests, lever->chest
    // mapping, mushroom-up destinations. None of it is reachable from the new `Dungeon`.
    server.dungeon = new_dungeon;

    // Fresh randomness for the new run's room/mob-layout selection, not a continuation of the
    // old run's RNG stream.
    SeededRng::set_seed(rand::random());

    // --- 3. Build the new dungeon's world state (blocks, Mort, locked chests, doors, ...) ---
    crate::populate_dungeon_world(server)?;

    // The Entrance room's own spawn point, exactly as set for a brand-new player joining.
    let teleport_position = server.world.spawn_point;
    let spawn_yaw = server.world.spawn_yaw;
    let spawn_pitch = server.world.spawn_pitch;

    // --- 4. Resync every connected player atomically: old entities gone, new dungeon in view ---
    for (_, player) in server.world.players.iter_mut() {
        player.write_packet(&DestroyEntites {
            entities: old_entity_ids.iter().map(|id| VarInt(*id)).collect(),
        });

        // Must happen before the new Mort's spawn packets go out below (via
        // `resync_chunk_range`), which reuse the same UUID/team name every time.
        remove_stale_mort_client_state(player);

        // The player's map item (always id 1) is client-side texture-cached independently of
        // anything else - the fresh `DungeonMap` built above only tracks pixels changed *from
        // here on*, so without an explicit full wipe the client keeps showing whatever rooms
        // the old dungeon had already drawn (e.g. its entrance) alongside the new dungeon's own
        // drawing, since the two rarely land on the exact same pixels. A full 128x128 blank
        // send resets the client's cached texture completely, matching a genuinely fresh map.
        player.write_packet(&Maps {
            id: 1,
            scale: 0,
            columns: 128,
            rows: 128,
            x: 0,
            z: 0,
            map_data: vec![0u8; 128 * 128],
        });

        player.position = teleport_position;
        player.last_position = teleport_position;
        player.yaw = spawn_yaw;
        player.last_yaw = spawn_yaw;
        player.pitch = spawn_pitch;
        player.last_pitch = spawn_pitch;
        player.write_packet(&PositionLook {
            x: teleport_position.x,
            y: teleport_position.y,
            z: teleport_position.z,
            yaw: spawn_yaw,
            pitch: spawn_pitch,
            flags: 0,
        });

        // Anything scoped to the old run specifically, not the player's session as a whole.
        player.current_room_index = None;
        player.dungeon_stats = DungeonPlayerStats::default();
        player.has_redstone_key = false;
        player.has_wither_key = false;
        player.has_blood_key = false;
        player.dungeon_entry_tick = None;
        player.current_terminal = None;
        // Only force-close an actual server-tracked container (Mort's menu, CNC, a p3
        // terminal) - `UI::Inventory`/`UI::None` aren't real open windows server-side, so
        // there's nothing to send a `CloseWindow` for.
        if player.current_ui.get_container_data().is_some() {
            player.current_ui = UI::None;
            player.write_packet(&CloseWindow {
                window_id: player.window_id,
            });
        }

        let world = player.world_mut();
        resync_chunk_range(world, player, min_chunk_x, max_chunk_x, min_chunk_z, max_chunk_z);

        player.flush_packets();
    }

    // Discard the block/entity-spawn packet noise `populate_dungeon_world` generated while
    // building the new dungeon (every affected chunk's `packet_buffer`) - already delivered
    // explicitly above via `resync_chunk_range`, so leaving it would just get redundantly
    // re-sent (duplicate SpawnPlayer/Teams packets for Mort included) on the next normal tick.
    discard_chunk_packet_noise(&mut server.world, min_chunk_x, max_chunk_x, min_chunk_z, max_chunk_z);

    Ok(())
}

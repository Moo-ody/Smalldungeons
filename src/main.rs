mod dungeon;
mod net;
#[cfg(test)]
mod perf_bench;
mod server;
mod utils;

use crate::dungeon::door::DoorType::{self, WITHER};
use crate::dungeon::dungeon::Dungeon;
use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::room::room_data::{RoomData, RoomType};
use crate::dungeon::room::tic_tac_toe::CellState::X;
// use crate::dungeon::room::room::Room;
use crate::net::internal_packets::{MainThreadMessage, NetworkThreadMessage};
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound;
use crate::net::protocol::play::clientbound::{AddEffect, PlayerListItem, Teams};
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::net::run_network::run_network_thread;
use crate::net::var_int::VarInt;
use crate::server::block::block_collision::get_block_aabb;
use crate::server::block::block_parameter::Axis::Z;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::chunk::chunk::Chunk;
use crate::server::chunk::chunk_grid::ChunkDiff;
use crate::server::entity::dungeon_mobs::mob_type;
use crate::server::entity::dungeon_mobs::mob_type::DungeonMobType::ZombieLord;
use crate::server::entity::entity::{Entity, EntityImpl, NoEntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::entity::spawn_equipped::spawn_following_nametag;
// use crate::server::lava_boost::apply_lava_boost;
use crate::server::player::container_ui::UI;
use crate::server::player::player::{Player, GameProfile, GameProfileProperty};
use crate::server::utils::player_list::player_profile::{PlayerData, GameType};
use crate::server::player::scoreboard::{ScoreboardLines, CREATE_TEAM, ADD_PLAYER};
use crate::server::server::Server;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::server::utils::dvec3::{self, DVec3};
use crate::server::utils::sized_string::SizedString;
use crate::server::world::{self, VIEW_DISTANCE, World};
use crate::utils::hasher::deterministic_hasher::DeterministicHashMap;
use crate::utils::seeded_rng::{SeededRng, seeded_rng};
use anyhow::Result;
use chrono::Local;
use chrono::format::Pad::Zero;
use include_dir::include_dir;
use indoc::formatdoc;
use rand::seq::IndexedRandom;
use std::collections::HashMap;
use std::env;
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::unbounded_channel;
use uuid::Uuid;

// Mort skin constants. Slim (Alex) model - the base skin renders correctly with the classic
// (Steve, wide-arm) model at rest, but the wide arm model overlaps the sleeve overlay layer's
// slim-authored geometry, producing a visible black bar alongside the arms (a common MC skin
// issue: any skin actually authored for the slim arm width shows this when forced onto the
// classic model). Re-signed via mineskin.org's `/v2/generate` with `variant: "slim"` against the
// same underlying texture hash (`9b56895...` - same image, just re-tagged and re-signed by
// Mojang) rather than hand-editing the property, since a forged/mismatched signature on a
// player-model NPC silently falls back to the default Steve skin instead of rendering at all.
const MORT_SKIN_VALUE: &str = "ewogICJ0aW1lc3RhbXAiIDogMTYxODc4MTA4Mzk0NywKICAicHJvZmlsZUlkIiA6ICJhNzdkNmQ2YmFjOWE0NzY3YTFhNzU1NjYxOTllYmY5MiIsCiAgInByb2ZpbGVOYW1lIiA6ICIwOEJFRDUiLAogICJzaWduYXR1cmVSZXF1aXJlZCIgOiB0cnVlLAogICJ0ZXh0dXJlcyIgOiB7CiAgICAiU0tJTiIgOiB7CiAgICAgICJ1cmwiIDogImh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvOWI1Njg5NWI5NjU5ODk2YWQ2NDdmNTg1OTkyMzhhZjUzMmQ0NmRiOWMxYjAzODliOGJiZWI3MDk5OWRhYjMzZCIsCiAgICAgICJtZXRhZGF0YSIgOiB7CiAgICAgICAgIm1vZGVsIiA6ICJzbGltIgogICAgICB9CiAgICB9CiAgfQp9";
const MORT_SKIN_SIGNATURE: &str = "aNIhT2Tj20v1lONBOK3fIwBqJwWnjErq20h663Gb+PVmR9Iweh1h2ZEJ2pwDDnM4Af1XFDA5hS1Z9yOc8EdVTKyyi1yj9EIvMwQz/Q4N2sBsjWGZtCe8/Zy+X82iv0APB4cumE2gkgDbPjxCFNbpVKmV3U1WzwY/GKOMHofhWS1ULedQ1TszuMmDuHPLEzWaXigZ+xt5zChXvE8QoLTfBvgb8wtqVpyxAKf/o8xQduKiNE7t+de1CwOhLqbVTGh7DU0vLC5stDuqN+nC9dS7c2CG0ori6gFoGMvP4oIss6zm1nb0laMrZidJTgmuXk2Pv4NGDBXdYcAzhfWcSWGsBVMWrJfccgFheG+YcGYaYj6V2nBp0YTqqhN4wDt3ltyTNEMOr/JKyBTLzq/F7IL6rrdyMw+MbAgCa1FhfXxtzdQE2KsL55pbr2DZ8J4DYf+/OC1pWCJ4vvA/A1qGHyi3Zwtj9lCl1Jq5Qm2P9BgWxpk0ikJefRPMg4qWOEcYnjqwXuEp+IgTJi1xr+j/+g28aS1TsF8ijaJjSbEN4urrf3RYL+PZBcggzX9VaPB0NPdioOXznIotY+S6ZW7FnSh6UnrGAKadQBVLey5zmVWMfXlBUq9JMh0csuNd4dDQCLNK8oGORhMgksOMHhVaBie4otUgJ7ThR/WPjOAKiG2TNU0=";
/// Fixed, deterministic UUID reused for the Mort NPC across every dungeon (both at boot and
/// every `dungeon_switch::switch_dungeon` rebuild). Because it never changes, a client that
/// already has a tab-list entry/hidden-nameplate team for this UUID from a *previous* Mort
/// must have both explicitly removed before a new Mort spawns - see
/// `dungeon_switch::remove_stale_mort_client_state` - or the client sees a duplicate
/// `ADD_PLAYER`/`CREATE_TEAM` for already-existing state, which can corrupt its packet
/// handling for everything else in the same burst (this was the "everything entity-related
/// breaks after a dungeon switch" bug).
pub const MORT_UUID_STR: &str = "550e8400-e29b-41d4-a716-446655440000";

/// The Mort NPC's `EntityImpl`. Module-level (not defined inline in `populate_dungeon_world`)
/// so it can be constructed both at boot and every time `dungeon_switch::switch_dungeon`
/// rebuilds the dungeon from scratch.
pub struct MortImpl {
    /// `Entity::ticks_existed` of the last interaction we acted on, so a single
    /// right-click that arrives as two `UseEntity` packets (InteractAt + Interact)
    /// only opens the menu once. `u32::MAX` = "never interacted yet".
    last_interact_tick: u32,
}

impl EntityImpl for MortImpl {
    fn spawn(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        // Add Mort to player list so the player entity can be spawned properly
        if let Some(uuid) = entity.uuid {
            let mort_profile = GameProfile {
                uuid,
                username: "Mort".to_string(),
                properties: HashMap::from([
                    ("textures".to_string(), GameProfileProperty {
                        value: MORT_SKIN_VALUE.to_string(),
                        signature: Some(MORT_SKIN_SIGNATURE.to_string()),
                    })
                ]),
            };

            let player_data = PlayerData {
                ping: 20, // More realistic ping for NPCs
                game_mode: GameType::Survival, // Use Survival instead of Creative for better modded client compatibility
                profile: mort_profile,
                display_name: Some(ChatComponentTextBuilder::new("Mort").build()),
            };

            // Send PlayerInfo Add packet
            buffer.write_packet(&PlayerListItem {
                action: VarInt(0), // ADD_PLAYER
                players: vec![&player_data],
            });

            // First create the hidden team to prevent vanilla nameplate from showing
            buffer.write_packet(&Teams {
                name: SizedString::truncated("npc_hide"),
                display_name: SizedString::truncated("npc_hide"),
                prefix: SizedString::truncated(""),
                suffix: SizedString::truncated(""),
                name_tag_visibility: SizedString::truncated("never"),
                color: 0,
                players: vec![],
                action: CREATE_TEAM,
                friendly_flags: 0,
            });

            // Then add Mort to the hidden team
            buffer.write_packet(&Teams {
                name: SizedString::truncated("npc_hide"),
                display_name: SizedString::truncated("npc_hide"),
                prefix: SizedString::truncated(""),
                suffix: SizedString::truncated(""),
                name_tag_visibility: SizedString::truncated("never"),
                color: 0,
                players: vec![SizedString::truncated("Mort")],
                action: ADD_PLAYER,
                friendly_flags: 0,
            });
        }
    }

    fn tick(&mut self, entity: &mut Entity, _: &mut PacketBuffer) {
        // Same "personal space" facing reaction dungeon mobs get while idle (see
        // `ai::mod::IDLE_FACE_PLAYER_RANGE`/its handling in `run_mob_ai`) - Mort isn't on the
        // dungeon-mob AI pipeline at all (no combat, no wandering), so this is his only motion.
        const MORT_FACE_PLAYER_RANGE: f64 = 8.0;
        let mort_pos = entity.position;
        let world = entity.world_mut();
        let nearest_player_pos = world.players.values()
            .map(|player| (player.position, player.position.distance_to(&mort_pos)))
            .filter(|(_, dist)| *dist <= MORT_FACE_PLAYER_RANGE)
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(pos, _)| pos);

        if let Some(player_pos) = nearest_player_pos {
            crate::server::entity::dungeon_mobs::ai::movement::face_toward(entity, player_pos);
        }
    }
    fn interact(&mut self, entity: &mut Entity, player: &mut Player, action: &EntityInteractionType) -> bool {
        // Right-clicking an entity is how you talk to Mort on Hypixel. In the 1.8
        // protocol a right-click on an entity is delivered as `InteractAt` (type 2,
        // carries the hit vector) - NOT `Interact` (type 0) - so the old code, which
        // returned early on `InteractAt` and only opened on `Interact`, never fired on
        // an actual right-click. Left-click (`Attack`) is not a valid way to open Mort,
        // so it's ignored here rather than used as a stand-in.
        if *action == EntityInteractionType::Attack {
            return false;
        }
        // Some clients send both `InteractAt` and `Interact` for one right-click;
        // debounce on the tick so a single click opens the menu only once. Still report
        // `true` on the debounced duplicate - it's the same click as the one that opened
        // the menu, so the held item shouldn't also fire for it.
        if entity.ticks_existed == self.last_interact_tick {
            return true;
        }
        self.last_interact_tick = entity.ticks_existed;
        player.open_ui(UI::MortReadyUpMenu);
        true
    }
}

/// Maps this codebase's puzzle room names to OdinClient's `Puzzle` enum `displayName` strings
/// (`DungeonEnums.kt`, confirmed from OdinClient's own source) - identical to the room name for
/// every puzzle already scraped except "Blaze", which Odin's enum calls "Higher Or Lower".
/// `None` for anything that isn't a recognized puzzle room.
fn puzzle_odin_display_name(room_name: &str) -> Option<&'static str> {
    match room_name {
        "Blaze" => Some("Higher Or Lower"),
        "Boulder" => Some("Boulder"),
        "Creeper Beams" => Some("Creeper Beams"),
        "Ice Fill" => Some("Ice Fill"),
        "Ice Path" => Some("Ice Path"),
        "Quiz" => Some("Quiz"),
        "Teleport Maze" => Some("Teleport Maze"),
        "Three Weirdos" => Some("Three Weirdos"),
        "Tic Tac Toe" => Some("Tic Tac Toe"),
        "Water Board" => Some("Water Board"),
        _ => None,
    }
}

/// Loads every room's blocks into the world, spawns Mort in the entrance room and sets the
/// world spawn point, spawns locked chests, applies a handful of special per-room block
/// tweaks, and loads every door's blocks. Assumes `server.dungeon` is already a freshly built
/// `Dungeon` (rooms/doors populated, no blocks placed into `server.world` yet) - used both by
/// `main()` at boot and by `dungeon_switch::switch_dungeon` to repopulate after a full reset.
pub fn populate_dungeon_world(server: &mut Server) -> anyhow::Result<()> {
    let dungeon = &mut server.dungeon;

    for (room_index, room) in dungeon.rooms.iter_mut().enumerate() {
        // println!("Room: {:?} type={:?} rotation={:?} shape={:?} corner={:?}", room.segments, room.room_data.room_type, room.rotation, room.room_data.shape, room.get_corner_pos());
        room.load_into_world(room_index, &mut server.world);
        // Mobs are spawned when a player actually enters the room (see Dungeon::tick /
        // Dungeon::start_dungeon), not eagerly here for the whole dungeon at once.

        // Immediately scan crypts on world load
        if room.crypt_patterns.len() > 0 && !room.crypts_checked {
            room.detect_crypts(&server.world);
        }


        // Set the spawn point to be inside of the spawn room
        if room.room_data.room_type == RoomType::Entrance {
            server.world.set_spawn_point(
                room.get_world_block_pos(&BlockPos {
                    x: 15,
                    y: 72,
                    z: 18,
                })
                .as_dvec3()
                .add_x(0.5)
                .add_z(0.5),
                180.0.rotate(room.rotation),
                0.0,
            );

            // Generate deterministic UUID for Mort NPC using parse_str with a fixed UUID
            let mort_uuid = Uuid::parse_str(MORT_UUID_STR).unwrap(); // Mort NPC UUID

            let id = server.world.spawn_entity_with_uuid(
                room.get_world_block_pos(&BlockPos { x: 15, y: 69, z: 4 })
                    .as_dvec3()
                    .add(DVec3::new(0.5, 0.0, 0.5)),
                EntityMetadata::new(EntityVariant::Player),
                MortImpl { last_interact_tick: u32::MAX },
                Some(mort_uuid),
            )?;
            if let Some((entity, _)) = server.world.entities.get_mut(&id) {
                entity.yaw = 0.0.rotate(room.rotation);
            }

            // Create two-line armorstand nametag for Mort
            // Get Mort's position first
            let mort_pos = if let Some((entity, _)) = server.world.entities.get(&id) {
                entity.position
            } else {
                // Fallback to the original position if entity not found yet
                room.get_world_block_pos(&BlockPos { x: 15, y: 69, z: 4 })
                    .as_dvec3()
                    .add(DVec3::new(0.5, 0.0, 0.5))
            };

            // Try following nametags first
            // Bumped up from 0.4/0.1 - `spawn_following_nametag`'s armor stand now renders at
            // ~half scale ("small" status bit, added for the starred-mob-box height fix), so
            // its own model contributes much less passive height than before, and Mort's
            // offsets (tuned under the old full-size assumption) ended up sitting too low.
            // Mort isn't a starred mob so there's no box constraint here - free to just
            // compensate directly.
            // 1.3/1.0 (bumped from the original 0.4/0.1) came back "a bit high" - splitting the
            // difference down a bit rather than all the way back.
            match spawn_following_nametag(&mut server.world, id, "§bMort", 1.0, EntityVariant::ArmorStand) {
                Ok(_top_nametag_id) => {
                    // Spawn bottom nametag
                    match spawn_following_nametag(&mut server.world, id, "§eCLICK", 0.75, EntityVariant::ArmorStand) {
                        Ok(_bottom_nametag_id) => {
                            // Both nametags spawned successfully
                        }
                        Err(e) => {
                            println!("Failed to spawn bottom nametag: {}", e);
                            // Fallback to static armorstands
                            let _bottom_nametag_id = server.world.spawn_entity(
                                mort_pos + DVec3::new(0.0, 0.1, 0.0),
                                {
                                    let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
                                    metadata.is_invisible = true;
                                    metadata.custom_name = Some("§eCLICK".to_string());
                                    metadata.custom_name_visible = true;
                                    metadata.ai_disabled = true;
                                    metadata
                                },
                                NoEntityImpl,
                            )?;
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to spawn top nametag: {}", e);
                    // Fallback to static armorstands
                    let _top_nametag_id = server.world.spawn_entity(
                        mort_pos + DVec3::new(0.0, 0.4, 0.0),
                        {
                            let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
                            metadata.is_invisible = true;
                            metadata.custom_name = Some("§bMort".to_string());
                            metadata.custom_name_visible = true;
                            metadata.ai_disabled = true;
                            metadata
                        },
                        NoEntityImpl,
                    )?;

                    let _bottom_nametag_id = server.world.spawn_entity(
                        mort_pos + DVec3::new(0.0, 0.1, 0.0),
                        {
                            let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
                            metadata.is_invisible = true;
                            metadata.custom_name = Some("§eCLICK".to_string());
                            metadata.custom_name_visible = true;
                            metadata.ai_disabled = true;
                            metadata
                        },
                        NoEntityImpl,
                    )?;
                }
            }

            // Ensure entity is properly visible first, then add team hiding if needed
            if let Some((_entity, _)) = server.world.entities.get(&id) {
                // Player entity should be properly registered now
                // For modded clients, we might want to ensure no immediate team modifications
                // that could interfere with visibility
            }
        }
    }

    // Spawn locked chests for all rooms
    for room in &dungeon.rooms {
        room.spawn_locked_chests(
            &mut server.world,
            &mut dungeon.locked_chests,
            &mut dungeon.lever_to_chests,
        );
    }

    // One random locked chest per dungeon becomes the mimic chest: same block/appearance as a
    // normal chest, swapped to `TrappedChest` (still visually identical - the real vanilla
    // block just has a different clientbound id) and taken out of the lock/lever system
    // entirely, since a mimic that also needed a key would give itself away. Trap rooms are
    // excluded - they already have their own trap mechanic, so a mimic there would be piling
    // one gimmick on top of another rather than a normal chest room's single surprise.
    let mimic_eligible_chests: Vec<&BlockPos> = dungeon.locked_chests.keys()
        .filter(|&&pos| {
            match dungeon.get_room_at(pos.x, pos.z).and_then(|idx| dungeon.rooms.get(idx)) {
                Some(room) => room.room_data.room_type != RoomType::Trap,
                None => true,
            }
        })
        .collect();
    if let Some(&&mimic_pos) = mimic_eligible_chests.choose(&mut seeded_rng()) {
        if let Some(state) = dungeon.locked_chests.remove(&mimic_pos) {
            dungeon.lever_to_chests.entry(state.lever_world_pos).and_modify(|chests| {
                chests.retain(|&pos| pos != mimic_pos);
            });

            let direction = match server.world.get_block_at(mimic_pos.x, mimic_pos.y, mimic_pos.z) {
                crate::server::block::blocks::Blocks::Chest { direction } => direction,
                _ => crate::server::utils::direction::Direction::North,
            };
            server.world.set_block_at(
                crate::server::block::blocks::Blocks::TrappedChest { direction },
                mimic_pos.x, mimic_pos.y, mimic_pos.z,
            );
            server.world.interactable_blocks.insert(
                mimic_pos,
                crate::server::block::block_interact_action::BlockInteractAction::MimicChest,
            );
        }
    }

    // Remove vines from specific rooms and add special blocks after all rooms are loaded
    for room in &dungeon.rooms {
        let corner = room.get_corner_pos();

        if room.room_data.name == "Rails" {
            // Remove vines at 15 58 15, 15 57 15, 15 56 15, 15 55 15, 15 54 15 (no rotation)
            let vines_to_remove = [
                (15, 58, 15),
                (15, 57, 15),
                (15, 56, 15),
                (15, 55, 15),
                (15, 54, 15),
            ];

            for (rel_x, rel_y, rel_z) in &vines_to_remove {
                let vine_world_pos = BlockPos {
                    x: corner.x + *rel_x,
                    y: *rel_y,
                    z: corner.z + *rel_z,
                };

                // Remove the vine block
                server.world.set_block_at(
                    Blocks::Air,
                    vine_world_pos.x,
                    vine_world_pos.y,
                    vine_world_pos.z
                );
            }
        } else if room.room_data.name == "Flags" {
            // Remove vine at 55 88 47 (no rotation)
            let vine_world_pos = BlockPos {
                x: corner.x + 55,
                y: 88,
                z: corner.z + 47,
            };

            // Remove the vine block
            server.world.set_block_at(
                Blocks::Air,
                vine_world_pos.x,
                vine_world_pos.y,
                vine_world_pos.z
            );
        } else if room.room_data.name == "Grand Library" {
            // Spawn torch at 47 86 15 (no rotation, like vines)
            let torch_world_pos = BlockPos {
                x: corner.x + 47,
                y: 86,
                z: corner.z + 15,
            };

            // Spawn the torch block (default direction is up - placed on top of block below)
            server.world.set_block_at(
                Blocks::Torch {
                    direction: crate::server::block::block_parameter::TorchDirection::Up,
                },
                torch_world_pos.x,
                torch_world_pos.y,
                torch_world_pos.z
            );
        }
    }

    // Lever system is now integrated into room generation (like crypts and superboom walls)

    for door in &dungeon.doors {
        door.load_into_world(&mut server.world, &server.door_type_blocks);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let (network_tx, network_rx) = unbounded_channel::<NetworkThreadMessage>();
    let (main_tx, mut main_rx) = unbounded_channel::<MainThreadMessage>();

    let args: Vec<String> = env::args().collect();

    let rooms_dir = include_dir!("src/room_data/rooms/");

    // Load secrets from bettermapRooms.json
    let bettermap_rooms_json = include_str!("room_data/rooms/bettermapRooms.json");
    let bettermap_rooms: serde_json::Value = serde_json::from_str(bettermap_rooms_json).unwrap();
    let mut secrets_map: std::collections::HashMap<String, u8> = std::collections::HashMap::new();
    
    if let Some(rooms_array) = bettermap_rooms.as_array() {
        for room in rooms_array {
            if let Some(name) = room.get("name").and_then(|n| n.as_str()) {
                if let Some(secrets) = room.get("secrets")
                    .and_then(|s| s.as_number())
                    .and_then(|n| n.as_u64())
                    .map(|n| n as u8) {
                    secrets_map.insert(name.to_string(), secrets);
                }
            }
        }
    }

    // The key here is just an opaque per-entry id - `get_random_data_with_type` (the only real
    // consumer during dungeon generation) filters by `room_type`/`shape` and picks randomly, never
    // by this key, so it doesn't need to mean anything. It used to be parsed from the file name's
    // room-id prefix instead of a fresh index - that silently dropped any room with two physical
    // variants sharing the same prefix (confirmed real: both "Blaze" room files, Higher/Lower,
    // are "10,blaze,...json" - whichever loaded second silently overwrote the first in the map,
    // so only one of the two ever actually existed at runtime). A sequential index can't collide.
    let room_data_storage: DeterministicHashMap<usize, RoomData> = rooms_dir
        .entries()
        .iter()
        .filter_map(|file| {
            let file = file.as_file()?;
            let file_name = file.path().file_name()?.to_str()?;

            // Skip bettermapRooms.json - it's not a room file
            if file_name == "bettermapRooms.json" {
                return None;
            }

            let contents = file.contents_utf8()?;
            let mut room_data = RoomData::from_raw_json(contents);

            // Override secrets from bettermapRooms.json if available
            if let Some(secrets) = secrets_map.get(&room_data.name) {
                room_data.secrets = *secrets;
            }

            // Override specific room secrets
            if room_data.name == "Golden Oasis" {
                room_data.secrets = 3;
            }

            Some(room_data)
        })
        .enumerate()
        .collect();

    // Load lever data - using include_str for now since the directory name has spaces
    let _lever_json_data = include_str!("room_data/misc/lever.json");

    // Might be a good idea to make a new format for storing doors so that indexes etc don't need to be hard coded.
    // But this works for now...
    let door_data: Vec<Vec<Blocks>> = include_str!("door_data/doors.txt")
        .split("\n")
        .map(|line| {
            let mut blocks: Vec<Blocks> = Vec::new();

            for i in (0..line.len() - 1).step_by(4) {
                if let Some(substr) = line.get(i..i + 4) {
                    if let Ok(state) = u16::from_str_radix(substr, 16) {
                        blocks.push(Blocks::from(state));
                    }
                }
            }

            blocks
        })
        .collect();

    let door_type_blocks: HashMap<DoorType, Vec<Vec<Blocks>>> = HashMap::from_iter(
        vec![
            (DoorType::BLOOD, vec![door_data[0].clone()]),
            (DoorType::ENTRANCE, vec![door_data[1].clone()]),
            (DoorType::WITHER, vec![
                door_data[2].clone(),
                door_data[3].clone(),
                door_data[4].clone(),
            ]),
            (
                DoorType::NORMAL,
                vec![
                    door_data[1].clone(),
                    door_data[2].clone(),
                    door_data[3].clone(),
                    door_data[4].clone(),
                    door_data[5].clone(),
                    door_data[6].clone(),
                    door_data[7].clone(),
                ],
            ),
        ]
        .into_iter(),
    );

    let dungeon_strings = include_str!("dungeon_storage/dungeons.txt")
        .split("\n")
        .collect::<Vec<&str>>();

    // Practice mode is a separate launch mode (`cargo run -- practice`, or the more
    // flag-shaped `--practice` - both accepted), not an in-game toggle: no dungeon layout is
    // generated at all - the world starts with zero rooms, and the first
    // `/practice <room> <door>` command builds one (see `dungeon::practice`). Anything else
    // in that argument slot falls through to being treated as a dungeon layout string, same as
    // before practice mode existed - a typo'd flag landing there would otherwise silently
    // panic deep in `Dungeon::from_str`'s layout parsing instead of doing what was intended.
    let practice_mode = matches!(args.get(1).map(|s| s.as_str()), Some("practice") | Some("--practice"));

    let dungeon = if practice_mode {
        println!("Launching in PRACTICE mode - use /practice <room> <door> in-game to load a room.");
        Dungeon::from_layout(Vec::new(), Vec::new())?
    } else {
        // Check if a custom dungeon str has been given via cli args

        // let dungeon_str = "080809010400100211121300101415161304171418161300191403161304191905160600919999113099910991099909090099999919990929999999099999999009";

        let dungeon_str = match args.len() {
            0..=1 => {
                let mut rng = rand::rng();
                dungeon_strings.choose(&mut rng).unwrap_or(&"080809010400100211121300101415161304171418161300191403161304191905160600919999113099910991099909090099999919990929999999099999999009")
            }
            _ => args.get(1).map(|s| s.as_str()).unwrap_or("080809010400100211121300101415161304171418161300191403161304191905160600919999113099910991099909090099999919990929999999099999999009"),
        };
        println!("Dungeon String: {}", dungeon_str);

        let rng_seed: u64 = rand::random(); // using a second seed for rng enables the same layout to have randomized rooms. Maybe should be included in the dungeon seed string?
        // let rng_seed: u64 = 12946977352813673410;

        println!("Rng Seed: {}", rng_seed);
        SeededRng::set_seed(rng_seed);

        Dungeon::from_str(dungeon_str, &room_data_storage)?
    };

    let mut server = Server::initialize_with_dungeon(network_tx, dungeon, room_data_storage, door_type_blocks);
    server.world.server = &mut server;
    server.dungeon.server = &mut server;
    server.practice_mode = practice_mode;
    if practice_mode {
        // No entrance room exists yet to set a real spawn point from - park players somewhere
        // safe above the (currently empty) dungeon grid until the first `/practice` runs.
        server.world.set_spawn_point(DVec3::new(15.5, 75.0, 15.5), 0.0, 0.0);
    }

    let mut tick_interval = tokio::time::interval(Duration::from_millis(50));
    tokio::spawn(run_network_thread(
        network_rx,
        server.network_tx.clone(),
        main_tx,
    ));

    // Load the bossroom at fixed coordinates first
    // {
    //     // Load the bossroom JSON data
    //     let bossroom_json = include_str!("room_data/146,bossroom,-8,-8.json");
    //     let bossroom_data = RoomData::from_raw_json(bossroom_json);
    //     
    //     // Extract dimensions before using bossroom_data
    //     let bossroom_width = bossroom_data.width;
    //     let bossroom_length = bossroom_data.length;
    //     let bossroom_height = bossroom_data.height;
    //     
    //     // Store boss room dimensions in dungeon for detection
    //     server.dungeon.boss_room_width = bossroom_width;
    //     server.dungeon.boss_room_length = bossroom_length;
    //     server.dungeon.boss_room_height = bossroom_height;
    //     
    //     // Create a single segment for the bossroom at the specified coordinates
    //     let bossroom_segments = vec![crate::dungeon::room::room::RoomSegment {
    //         x: 0, // This will be overridden by our custom positioning
    //         z: 0, // This will be overridden by our custom positioning
    //         neighbours: [None; 4],
    //     }];
    //     
    //     // Create the bossroom with North rotation (no rotation)
    //     let bossroom = Room::new(
    //         bossroom_segments,
    //         &[], // empty doors array
    //         bossroom_data,
    //     );
    //     
    //     // Override the corner position to spawn at -8, 254, -8
    //     // We need to manually load the bossroom since it's not part of the regular dungeon grid
    //     let corner = BlockPos { x: -8, y: 254, z: -8 };
    //     
    //     // Update the dungeon's boss room corner to match the actual loaded position
    //     server.dungeon.boss_room_corner = corner;
    //     
    //     // Manually load the bossroom blocks at the specified position
    //     for (i, block) in bossroom.room_data.block_data.iter().enumerate() {
    //         if *block == Blocks::Air {
    //             continue;
    //         }
    //         
    //         let block = block.clone();
    //         // No rotation needed since we're placing it directly
    //         
    //         let ind = i as i32;
    //         let x = ind % bossroom.room_data.width;
    //         let z = (ind / bossroom.room_data.width) % bossroom.room_data.length;
    //         let y = bossroom.room_data.bottom + ind / (bossroom.room_data.width * bossroom.room_data.length);
    //         
    //         // Place the block at the world position
    //         server.world.set_block_at(block, corner.x + x, y, corner.z + z);
    //     }
    //     
    //     println!("Bossroom loaded at coordinates: x={}, y={}, z={} with dimensions: {}x{}x{}", 
    //         corner.x, corner.y, corner.z, 
    //         server.dungeon.boss_room_width, 
    //         server.dungeon.boss_room_length, 
    //         server.dungeon.boss_room_height);
    //     
    //     // Bossroom chunks will be sent to players individually when they get near it
    //     // No need to send to all players immediately - this saves bandwidth and memory
    //     let bossroom_chunk_x_min = corner.x >> 4;
    //     let bossroom_chunk_z_min = corner.z >> 4;
    //     let bossroom_chunk_x_max = (corner.x + bossroom.room_data.width) >> 4;
    //     let bossroom_chunk_z_max = (corner.z + bossroom.room_data.length) >> 4;
    //     
    //     let total_chunks = (bossroom_chunk_x_max - bossroom_chunk_x_min + 1) * (bossroom_chunk_z_max - bossroom_chunk_z_min + 1);
    //     println!("Bossroom loaded with {} chunks ({}x{}), will be sent to players when they get near", 
    //         total_chunks, bossroom_chunk_x_max - bossroom_chunk_x_min + 1, bossroom_chunk_z_max - bossroom_chunk_z_min + 1);
    // }

    populate_dungeon_world(&mut server)?;

    // let zombie_spawn_pos = DVec3 {
    //     x: 25.0,
    //     y: 69.0,
    //     z: 25.0,
    // };

    // let zombie = Entity::create_at(EntityType::Zombie, zombie_spawn_pos, server.world.new_entity_id());
    // let path = Pathfinder::find_path(&zombie, &BlockPos { x: 10, y: 69, z: 10 }, &server.world)?;

    // server.world.entities.insert(zombie.entity_id, zombie);

    let cata_line = ChatComponentTextBuilder::new("")
        .append(
            ChatComponentTextBuilder::new("Dungeon: ")
                .color(MCColors::Aqua)
                .bold()
                .build(),
        )
        .append(
            ChatComponentTextBuilder::new("Catacombs")
                .color(MCColors::White)
                .build(),
        )
        .build();

    server.world.player_info.set_line(0, cata_line);

    loop {
        tick_interval.tick().await;
        // let start = std::time::Instant::now();

        while let Ok(message) = main_rx.try_recv() {
            server.process_event(message).unwrap_or_else(|err| eprintln!("Error processing event: {err}"));
        }

        server.dungeon.tick()?;
        server.world.tick()?;

        // for entity_id in server.world.entities.keys().cloned().collect::<Vec<_>>() {
        //     if let Some(mut entity) = server.world.entities.remove(&entity_id) {
        //         entity.ticks_existed += 1;
        //         // this may at some point be abused to prevent getting an entities own self if it iterates over world entities so be careful if you change this
        //         let returned = entity.update(&mut server.world, &server.network_tx);
        //         server.world.entities.insert(entity_id, returned);
        //     }
        // }

        let mut i: usize = 0;
        while i < server.tasks.len() {
            if server.tasks[i].run_in == 0 {
                let task = server.tasks.remove(i);
                (task.callback)(&mut server);
                // index isnt incremented since this entry was removed, shifting the next entry into its place.
            } else {
                server.tasks[i].run_in -= 1;
                i += 1;
            }
        }

        // OdinClient's dungeon stats (Secrets Found, Cleared %, Completed/Opened Rooms, Crypts,
        // Team Deaths, Time - everything `DungeonUtils`/MapInfo read) come from `updateDungeonStats`
        // in its `DungeonListener`, which is fed from `S38PacketPlayerListItem` - the TAB LIST,
        // not the sidebar scoreboard - confirmed by tracing the actual call site in the mod's
        // bytecode: it's reached from inside the tab-list-entry-parsing branch of `onPacket`,
        // not anything scoreboard-related. Each stat is its own fake tab-list row (same trick
        // `PlayerList`/`generate_default_lines` already uses for alphabetical ordering) whose
        // plain display-name text (colors are stripped before matching) exactly matches one of
        // Odin's `^ Secrets Found: (\d+)%$`-style regexes. Real values come from
        // `dungeon.score` (`DungeonScoreState`, src/dungeon/score.rs), the same live-ticked
        // scoring system behind the S/S+ chat announcements - not recomputed here.
        if let DungeonState::Started { current_ticks } = &server.dungeon.state {
            let elapsed_ticks = if server.dungeon.practice_room {
                if let Some(finish_seconds) = server.dungeon.practice_route_finish_seconds {
                    (finish_seconds * 20.0) as u64
                } else if let Some(start_tick) = server.dungeon.practice_route_start_tick {
                    current_ticks.saturating_sub(start_tick)
                } else {
                    0
                }
            } else {
                *current_ticks
            };
            let seconds = elapsed_ticks / 20;
            let odin_time = {
                let hours = seconds / 3600;
                let minutes = (seconds % 3600) / 60;
                let secs = seconds % 60;
                let mut parts = Vec::new();
                if hours > 0 {
                    parts.push(format!("{hours}h"));
                }
                if hours > 0 || minutes > 0 {
                    parts.push(format!("{minutes}m"));
                }
                parts.push(format!("{secs}s"));
                parts.join(" ")
            };

            let score = &server.dungeon.score;
            let clear_percent = if score.total_rooms == 0 {
                0
            } else {
                (score.cleared_rooms * 100) / score.total_rooms
            };
            // Odin's `secretPercentRegex` (`^ Secrets Found: ([\d.]+)%$`) allows a decimal point -
            // real Hypixel sends fractional precision here, and `DungeonUtils.getTotalSecrets()`
            // derives the total as `secretsFound / (secretsPercent / 100)`, which is extremely
            // sensitive to rounding at low counts. A truncated integer percent (e.g. 1/40 secrets
            // rounding to "3%" instead of "2.5%") threw that derivation off until enough secrets
            // accumulated to wash the error out - matches the "took 13 secrets to converge"
            // symptom. Two decimal places gets it right from the first secret found.
            let secrets_percent = if score.total_secrets == 0 {
                0.0
            } else {
                (score.secrets_found as f64 * 100.0) / score.total_secrets as f64
            };
            let cleared_rooms = score.cleared_rooms;
            let crypts = score.crypts;
            let deaths = score.deaths;
            let opened_rooms = server.dungeon.rooms.iter().filter(|room| room.entered).count();

            // Indices 1/5/9/13/17 are reserved for Skytils' Catlas below - Odin doesn't care
            // which fake tab-list row its stats land on (it matches by stripped text against its
            // own regexes, not position), so these were moved off that range rather than fight
            // over it. Anything clear of {1,5,9,13,17} works; 20-27 was just picked for headroom.
            // Colors are embedded directly as legacy `§` codes (same convention every chat
            // message in this codebase already uses via `send_message`) rather than a single
            // blanket `.color()` - Odin strips color before matching these against its regexes
            // (see the doc comment above), so embedding them here is safe. Per explicit color
            // scheme: labels gray, plain informational values white, completion-progress values
            // green, in-progress values yellow, deaths red, Secrets aqua, Crypts gold, and any
            // parenthetical extra detail (a raw count alongside a percentage) dark gray.
            let tab_stat_line = |text: String| ChatComponentTextBuilder::new(text).build();
            server.world.player_info.set_line(20, tab_stat_line(format!("\u{a7}7 Time: \u{a7}f{odin_time}")));
            server.world.player_info.set_line(21, tab_stat_line(format!("\u{a7}7Cleared: \u{a7}a{clear_percent}% \u{a7}8({cleared_rooms})")));
            server.world.player_info.set_line(22, tab_stat_line(format!("\u{a7}7 Completed Rooms: \u{a7}a{cleared_rooms}")));
            server.world.player_info.set_line(23, tab_stat_line(format!("\u{a7}7 Opened Rooms: \u{a7}e{opened_rooms}")));
            server.world.player_info.set_line(24, tab_stat_line(format!("\u{a7}7 Secrets Found: \u{a7}b{secrets_percent:.2}%")));
            server.world.player_info.set_line(25, tab_stat_line(format!("\u{a7}7 Crypts: \u{a7}6{crypts}")));
            server.world.player_info.set_line(26, tab_stat_line(format!("\u{a7}7Team Deaths: \u{a7}c{deaths}")));
            // MapInfo's actual secrets calculation (`MapInfo$compactSecrets$2`, traced directly)
            // needs BOTH `DungeonStats.secretsFound` (this raw-count line, `secretCountRegex` =
            // `^ Secrets Found: (\d+)$`) AND `secretsPercent` (the line above, `secretPercentRegex`
            // = `^ Secrets Found: ([\d.]+)%$`) - `getTotalSecrets()`'s formula is
            // `secretsFound / (secretsPercent / 100)`, so with secretsFound stuck at its default
            // 0 (no raw-count line ever sent before), the result was always 0 regardless of the
            // percent line - confirmed in bytecode, not assumed this time.
            server.world.player_info.set_line(27, tab_stat_line(format!("\u{a7}7 Secrets Found: \u{a7}b{}", score.secrets_found)));

            // Minimal stub so Skytils' Catlas can find real players at all: `DungeonListener`
            // (see `onPacket`'s `S38PacketPlayerListItem` branch, confirmed from Skytils' own
            // source) only recognizes a tab row as a real player if its position is one of the 5
            // it hardcodes (`playerEntryNames`: "!A-b"->1, "!A-f"->5, "!A-j"->9, "!A-n"->13,
            // "!A-r"->17 - matching `generate_default_lines`' fake-profile naming) AND its display
            // text matches `classPattern`, which requires a `(<Class> <Level>)` suffix - this
            // project has no real dungeon-class system, so every player gets a fixed placeholder
            // ("Mage L", per explicit request) purely to satisfy that parser; nothing about class
            // selection or leveling is real yet. Without at least this, Catlas's `team` map never
            // gets an entry for anyone - not even the local player - so no map pointer renders at
            // all, solo or otherwise. Sorted by player id for a stable slot assignment tick to tick.
            const CLASS_TAB_SLOTS: [usize; 5] = [1, 5, 9, 13, 17];
            let mut player_ids: Vec<u32> = server.world.players.keys().copied().collect();
            player_ids.sort_unstable();
            for (&slot, player_id) in CLASS_TAB_SLOTS.iter().zip(player_ids.iter()) {
                if let Some(player) = server.world.players.get(player_id) {
                    let username = &player.profile.username;
                    let text = format!("\u{a7}r\u{a7}a{username} \u{a7}r\u{a7}f(\u{a7}r\u{a7}dMage L\u{a7}r\u{a7}f)\u{a7}r");
                    server.world.player_info.set_line(slot, ChatComponentTextBuilder::new(text).build());
                }
            }

            // Puzzle-done reporting, placeholder rule: a puzzle counts as solved (✔) the instant
            // its room is entered - real per-puzzle-type solve/fail mechanics don't exist in this
            // codebase yet (see `score.rs`'s own doc comment), this is just enough for
            // OdinClient's `DungeonListener` to have something to read. It reads two things off
            // the tab list (`getDungeonPuzzles`/`updateDungeonStats`, confirmed from OdinClient's
            // own source): one line per puzzle, ` <PuzzleName>: [<status>]` (✦ discovered/
            // incomplete, ✔ completed, ✖ failed - only ✔ is ever produced here, since a room is
            // either not entered yet or "done" under this placeholder), matched by
            // `puzzleRegex` against `Puzzle::displayName`; and a total count line, `Puzzles:
            // (N)`, matched by `puzzleCountRegex`. `puzzle_odin_display_name` maps this
            // codebase's room names to Odin's exact `Puzzle` enum display strings - identical for
            // every puzzle room name already scraped except "Blaze", which Odin's enum calls
            // "Higher Or Lower".
            let puzzle_rooms: Vec<_> = server.dungeon.rooms.iter()
                .filter(|room| room.room_data.room_type == RoomType::Puzzle)
                .collect();
            server.world.player_info.set_line(28, tab_stat_line(format!("\u{a7}d\u{a7}lPuzzles: \u{a7}8({})", puzzle_rooms.len())));
            const PUZZLE_TAB_SLOTS: [usize; 6] = [29, 30, 31, 32, 33, 34];
            for (&slot, room) in PUZZLE_TAB_SLOTS.iter().zip(puzzle_rooms.iter()) {
                if !room.entered {
                    continue;
                }
                if let Some(display_name) = puzzle_odin_display_name(&room.room_data.name) {
                    // Odin's `puzzleRegex` (`DungeonListener.kt`) reads "✔" as Completed and
                    // "✖" as Failed - two genuinely distinct outcomes there, unlike the in-game
                    // map's checkmark (`DungeonMap::draw_room`), which shows the same glyph for
                    // both since a failed puzzle still counts as an explored/done room on the
                    // map. `room.entered` gating this whole block only tells us the room's been
                    // walked into, not resolved - a puzzle that's been entered but not yet
                    // solved or failed still shows the "done" glyph here, that placeholder gap
                    // predates this change and isn't part of what was reported.
                    // Colored per the explicit puzzle-status scheme: completed ✔ green, failed
                    // ✖ red (the not-yet-produced incomplete ✦ state would be yellow - see the
                    // doc comment above for why this placeholder never actually emits it).
                    let (glyph, glyph_color) = if room.puzzle_failed { ('\u{2716}', "\u{a7}c") } else { ('\u{2714}', "\u{a7}a") };
                    server.world.player_info.set_line(slot, tab_stat_line(format!("\u{a7}7 {display_name}: \u{a7}7[{glyph_color}{glyph}\u{a7}7]")));
                }
            }
        }

        let tab_list_packet = server.world.player_info.get_packet();

        // this needs to be changed to work with loaded chunks, tracking last sent data per player (maybe), etc.
        // also needs to actually be in a vanilla adjacent way.
        for player in server.world.players.values_mut() {
            player.ticks_existed += 1;

            // Trickle in any chunks still queued from a `sync_player_view` burst (join, or a
            // dungeon resync) - see `Player::pending_chunk_sync`'s doc comment for why this
            // isn't just sent all at once.
            crate::server::server::drain_pending_chunk_sync(player);

            // Send action bar every 5 ticks
            if player.ticks_existed % 5 == 0 {
                let stats = &player.dungeon_stats;
                let (found_secrets, total_secrets) = {
                    // Always try to get the current room
                    let room_index = server.dungeon.get_room_at(
                        player.position.x as i32,
                        player.position.z as i32
                    );
                    if let Some(room_index) = room_index {
                        // Update tracked room index
                        player.current_room_index = Some(room_index);
                        let dungeon = &server.dungeon;
                        if let Some(room) = dungeon.rooms.get(room_index) {
                            (room.found_secrets, room.room_data.secrets)
                        } else {
                            (0, 0)
                        }
                    } else {
                        // Player not in any room - clear tracked index
                        player.current_room_index = None;
                        (0, 0)
                    }
                };
                
                // Use section-sign approach (guaranteed to work in 1.8.9)
                let mut legacy_string = crate::server::player::dungeon_stats::build_action_bar_string(stats, found_secrets, total_secrets);
                // Practice-mode route timer (see `Dungeon::tick`) rides along in the same
                // action-bar packet rather than being sent separately - only one message can
                // occupy the action bar at a time, so a second packet would just overwrite this
                // one instead of showing alongside it.
                if server.dungeon.practice_room {
                    if let Some(timer_text) = &server.dungeon.practice_timer_text {
                        legacy_string = format!("{}   {}", legacy_string, timer_text);
                    }
                }
                let json_str = crate::server::player::dungeon_stats::legacy_to_actionbar_json(&legacy_string);
                
                // Parse JSON string into ChatComponentText
                let action_bar_component = match crate::server::utils::chat_component::chat_component_text::ChatComponentText::from_json_str(&json_str) {
                    Ok(component) => component,
                    Err(e) => {
                        eprintln!("Failed to parse action bar JSON: {} - Falling back to component approach", e);
                        // Fallback to component approach
                        crate::server::player::dungeon_stats::build_action_bar_component(stats, found_secrets, total_secrets)
                    }
                };
                
                player.write_packet(&clientbound::Chat {
                    component: action_bar_component,
                    chat_type: 2, // Position 2 = action bar
                });
            }
            player.write_packet(&clientbound::ConfirmTransaction {
                window_id: 0,
                action_number: -1,
                accepted: false,
            });

            // Continuously refresh SpiritSceptre in hotbar to prevent consumption
            if let Some(crate::server::player::inventory::ItemSlot::Filled(item, _)) = player.inventory.get_hotbar_slot(player.held_slot as usize) {
                if let crate::server::items::Item::SpiritSceptre = item {
                    let hotbar_slot = player.held_slot as usize + 36;
                    player.write_packet(&clientbound::SetSlot {
                        window_id: 0,
                        slot: hotbar_slot as i16,
                        item_stack: Some(item.get_item_stack()),
                    });
                }
            }

            let chunk_x = (player.position.x.floor() as i32) >> 4;
            let chunk_z = (player.position.z.floor() as i32) >> 4;
            let last_chunk_x = (player.last_position.x.floor() as i32) >> 4;
            let last_chunk_z = (player.last_position.z.floor() as i32) >> 4;

            let delta = (chunk_x - last_chunk_x, chunk_z - last_chunk_z);

            if delta.0 != 0 || delta.1 != 0 {
                // Chunks newly in view are queued into the same staggered `pending_chunk_sync`
                // system `sync_player_view` uses for the join-time burst, instead of being sent
                // synchronously right here - see that field's doc comment for the full story.
                // This diff fires for any position/chunk jump, including a big one-tick teleport
                // (Etherwarp, a teleport pad, `/practice`, ...) - sending potentially dozens of
                // chunks' worth of real block/light data in one synchronous burst, in one tick,
                // is exactly what a "TPS dip right when I teleport" symptom looks like: it's real
                // server-wide work blocking that tick for every player, not just this one.
                // Unloading (`ChunkDiff::Old`) stays immediate - a blank chunk has no real block
                // data to encode (`section_count` is 0), so it's cheap regardless of how many
                // chunks fall out of view at once. `/practice`'s teleport doesn't rely on this
                // staggering at all for its own destination - it calls
                // `dungeon_switch::resync_chunk_range` directly for a guaranteed-synchronous,
                // guaranteed-complete view (Odin's one-shot room scan needs real data the instant
                // it looks, not up to a second later), independent of this general path.
                let mut newly_visible: Vec<(i32, i32)> = Vec::new();
                server.world.chunk_grid.for_each_diff(
                    (chunk_x, chunk_z),
                    (last_chunk_x, last_chunk_z),
                    VIEW_DISTANCE as i32,
                    |x, z, diff| match diff {
                        ChunkDiff::New => {
                            newly_visible.push((x, z));
                        }
                        ChunkDiff::Old => {
                            // Entities are only ever (re)announced to a client via the
                            // ChunkDiff::New branch above, piggybacking on "this chunk is now
                            // in view" - there's no independent per-entity tracking. So a
                            // client that already knows about an entity here MUST be told to
                            // forget it before the chunk unloads, or the client's own
                            // spawn-once entity model means the *next* ChunkDiff::New for this
                            // chunk (re-sending SpawnMob/SpawnPlayer for the same still-alive
                            // entity ID) gets silently ignored as a duplicate - permanently
                            // invisible from then on, even though it's still alive server-side.
                            // This is a per-player packet only; the chunk's own `entities` list
                            // (shared, server-side bookkeeping of what's actually there) is
                            // untouched, since other players may still have this chunk in view.
                            if let Some(chunk) = player.world_mut().chunk_grid.get_chunk(x, z) {
                                if !chunk.entities.is_empty() {
                                    player.write_packet(&clientbound::DestroyEntites {
                                        entities: chunk.entities.iter().map(|id| VarInt(*id)).collect(),
                                    });
                                }
                            }
                            let chunk_data = Chunk::new().get_chunk_data(x, z, true);
                            player.write_packet(&chunk_data)
                        }
                    },
                );

                // Nearest-first, same convention `sync_player_view` uses - the chunk right next
                // to where the player now is matters more than the far edge of the new view.
                newly_visible.sort_by_key(|&(x, z)| (x - chunk_x).abs().max((z - chunk_z).abs()));
                player.pending_chunk_sync.extend(newly_visible);
            }

            {
                let view_distance = VIEW_DISTANCE as i32;
                let min_x = chunk_x - view_distance;
                let min_z = chunk_z - view_distance;
                let max_x = chunk_x + view_distance;
                let max_z = chunk_z + view_distance;

                for x in min_x..=max_x {
                    for z in min_z..=max_z {
                        if let Some(chunk) = player.world_mut().chunk_grid.get_chunk(x, z) {
                            player.packet_buffer.copy_from(&chunk.packet_buffer);
                        }
                    }
                }
            }

            let mut sidebar_lines = ScoreboardLines(Vec::new());

            // maybe match time with hypixel,
            let now = Local::now();
            let date = now.format("%m/%d/%y").to_string();
            let time = now.format("%-I:%M%P").to_string();

            let current_skyblock_month = {
                const SKYBLOCK_EPOCH_START_MILLIS: u64 = 1_559_829_300_000;
                const SKYBLOCK_YEAR_MILLIS: u64 = 124 * 60 * 60 * 1000;
                const SKYBLOCK_MONTH_MILLIS: u64 = SKYBLOCK_YEAR_MILLIS / 12;
                const SKYBLOCK_DAY_MILLIS: u64 = SKYBLOCK_MONTH_MILLIS / 31;

                const SKYBLOCK_MONTHS: [&str; 12] = [
                    "Early Spring", "Spring", "Late Spring",
                    "Early Summer", "Summer", "Late Summer",
                    "Early Autumn", "Autumn", "Late Autumn",
                    "Early Winter", "Winter", "Late Winter",
                ];

                let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
                let elapsed = now.saturating_sub(SKYBLOCK_EPOCH_START_MILLIS);
                let day = (elapsed % SKYBLOCK_YEAR_MILLIS) / SKYBLOCK_DAY_MILLIS;
                let month = (day / 31) as usize;
                let day_of_month = (day % 31) + 1;

                let suffix = match day_of_month % 100 {
                    11..=13 => "th",
                    _ => match day_of_month % 10 {
                        1 => "st",
                        2 => "nd",
                        3 => "rd",
                        _ => "th",
                    },
                };
                format!("{} {}{}", SKYBLOCK_MONTHS[month], day_of_month, suffix)
            };

            // Track when player first enters the dungeon (when dungeon becomes Started)
            let dungeon_state = &server.dungeon.state;
            if matches!(dungeon_state, DungeonState::Started { .. }) {
                if player.dungeon_entry_tick.is_none() {
                    // Player just entered the dungeon - record the current tick and hide scoreboard
                    if let DungeonState::Started { current_ticks } = dungeon_state {
                        player.dungeon_entry_tick = Some(*current_ticks);
                        // Hide the scoreboard when first entering
                        use crate::net::protocol::play::clientbound::DisplayScoreboard;
                        player.write_packet(&DisplayScoreboard {
                            position: -1, // -1 means hide the scoreboard
                            score_name: "SBScoreboard".into(),
                        });
                    }
                }
            } else {
                // Reset when not in dungeon
                player.dungeon_entry_tick = None;
            }
            
            // Check if we should skip scoreboard (first 5 ticks after joining dungeon)
            let skip_scoreboard = if let (Some(entry_tick), DungeonState::Started { current_ticks }) = 
                (player.dungeon_entry_tick, dungeon_state) {
                let ticks_since_entry = current_ticks.saturating_sub(entry_tick);
                ticks_since_entry < 5  // Skip for first 5 ticks (0-4)
            } else {
                false
            };
            
            // Skip scoreboard update for first 5 ticks after joining
            if skip_scoreboard {
                // Hide scoreboard every tick during the first 5 ticks
                use crate::net::protocol::play::clientbound::DisplayScoreboard;
                player.write_packet(&DisplayScoreboard {
                    position: -1, // -1 means hide the scoreboard
                    score_name: "SBScoreboard".into(),
                });
            } else {
                // Show scoreboard again after 5 ticks (only once when ticks_since_entry == 5)
                if let (Some(entry_tick), DungeonState::Started { current_ticks }) = 
                    (player.dungeon_entry_tick, dungeon_state) {
                    let ticks_since_entry = current_ticks.saturating_sub(entry_tick);
                    if ticks_since_entry == 5 {
                        // Show scoreboard again after 5 ticks
                        use crate::net::protocol::play::clientbound::DisplayScoreboard;
                        player.write_packet(&DisplayScoreboard {
                            position: 1, // 1 means show on sidebar
                            score_name: "SBScoreboard".into(),
                        });
                    }
                }
            // Normal scoreboard logic
            let show_simplified = false; // No longer using simplified scoreboard

            if show_simplified {
                // Simplified scoreboard for first 5 ticks (0.25s) when entering dungeon
                // Line 0: Header (title) - must be first
                sidebar_lines.push_str("SKYBLOCK");
                // Line 1: Date
                sidebar_lines.push_str(&format!("{date} m19BH"));
                // Line 2: Blank
                sidebar_lines.new_line();
                // Line 3: In-game day
                sidebar_lines.push_str(&current_skyblock_month);
                // Line 4: In-game time
                sidebar_lines.push_str(&time);
                // Line 5: Area line (None)
                sidebar_lines.push_str(" ⏣ None");
                // Line 6: Blank
                sidebar_lines.new_line();
                // Line 7: Website
                sidebar_lines.push_str("www.hypixel.net");
            } else {
                // Normal scoreboard
                let room_id = if let Some(room_index) = server.dungeon.get_player_room(player) {
                    if room_index < server.dungeon.rooms.len() {
                        let room = &server.dungeon.rooms[room_index];
                        
                        // removed periodic room bounds chat
                        
                        &room.room_data.id
                    } else {
                        eprintln!("Warning: Room index {} out of bounds for rooms vector of length {}", room_index, server.dungeon.rooms.len());
                        ""
                    }
                } else {
                    ""
                };

                sidebar_lines.push(formatdoc! {r#"
                    §e§lSKYBLOCK
                    §7{date} §8local {room_id}

                    {current_skyblock_month}
                    §7{time}
                     §7⏣ §cThe Catacombs §7(F7)

                "#});
            }

            // Only add dungeon state-specific lines if not showing simplified scoreboard
            if !show_simplified {
                match server.dungeon.state {
                    DungeonState::NotReady => {
                        for p in player.server_mut().world.players.values() {
                            sidebar_lines.push(format!("§c[M] §a{}", p.profile.username))
                        }
                        sidebar_lines.new_line();
                    }
                    DungeonState::Starting { tick_countdown } => {
                        for p in player.server_mut().world.players.values() {
                            sidebar_lines.push(format!("§a[M] §a{}", p.profile.username))
                        }
                        sidebar_lines.new_line();
                        sidebar_lines.push(format!("Starting in: §a0§a:0{}", (tick_countdown / 20) + 1));
                        sidebar_lines.new_line();
                    }
                    DungeonState::Started { current_ticks } => {
                        // this is scuffed but it works
                        //
                        // In practice mode the route timer doesn't start until a player first
                        // moves (see `Dungeon::tick`'s `practice_route_start_tick` gating) - the
                        // sidebar should reflect that instead of counting up from room load like
                        // a normal dungeon run does.
                        let elapsed_ticks = if server.dungeon.practice_room {
                            if let Some(finish_seconds) = server.dungeon.practice_route_finish_seconds {
                                (finish_seconds * 20.0) as u64
                            } else if let Some(start_tick) = server.dungeon.practice_route_start_tick {
                                current_ticks.saturating_sub(start_tick)
                            } else {
                                0
                            }
                        } else {
                            current_ticks
                        };
                        let seconds = elapsed_ticks / 20;
                        let time = if seconds >= 60 {
                            let minutes = seconds / 60;
                            let seconds = seconds % 60;
                            format!(
                                "{}{}m{}{}s",
                                if minutes < 10 { "0" } else { "" },
                                minutes,
                                if seconds < 10 { "0" } else { "" },
                                seconds
                            )
                        } else {
                            let seconds = seconds % 60;
                            format!("{}{}s", if seconds < 10 { "0" } else { "" }, seconds)
                        };
                        // TODO: display correct keys
                        //
                        // Cleared %/count feeds OdinClient's `DungeonListener.clearedRegex`
                        // (`^Cleared: (\d+)% \(\d+\)$`) - it reads this off the scoreboard
                        // sidebar via Team packets (`S3EPacketTeams` action 2, prefix+suffix),
                        // not the tab list the other Odin stats come from (confirmed against
                        // OdinClient's own source). Was hardcoded to 0/0 - same
                        // cleared_rooms/total_rooms derivation the tab-list "Cleared: X%" line
                        // already uses (see below), just fed into this line too.
                        let score = &server.dungeon.score;
                        let clear_percent = if score.total_rooms == 0 {
                            0
                        } else {
                            (score.cleared_rooms * 100) / score.total_rooms
                        };
                        let cleared_rooms = score.cleared_rooms;
                        sidebar_lines.push(formatdoc! {r#"
                            Keys: §c■ §c✖ §8■ §a0x
                            Time Elapsed: §a{time}
                            Cleared: §c{clear_percent}% §8({cleared_rooms})

                            §3§lSolo

                        "#});
                    }
                    DungeonState::Finished => {}
                }
            }

            if let Some(tab_list) = &tab_list_packet {
                player.write_packet(tab_list);
            }

            // Only add website line if not showing simplified scoreboard (already added there)
            if !show_simplified {
                sidebar_lines.push_str("§emc.hypixel.net");
            }

            if let Some(tab_list) = &tab_list_packet {
                player.write_packet(tab_list);
            }
            player.sidebar.write_update(sidebar_lines, &mut player.packet_buffer);
            } // End of skip_scoreboard else block

            if player.ticks_existed % 60 == 0 {
                player.write_packet(&AddEffect {
                    entity_id: VarInt(player.entity_id),
                    effect_id: 3,
                    amplifier: 2,
                    duration: VarInt(200),
                    hide_particles: true,
                });
                player.write_packet(&AddEffect {
                    entity_id: VarInt(player.entity_id),
                    effect_id: 16,
                    amplifier: 0,
                    duration: VarInt(400),
                    hide_particles: true,
                });
            }
            
            // Apply lava boost system (only in boss rooms)
            // let is_in_boss_room = server.dungeon.is_player_in_boss_room(player);
            // We need to check lava in the world, but we can't borrow world while player is mutably borrowed
            // So we'll pass the world reference through the player's world_mut method
            // let world_ref = player.world_mut();
            // apply_lava_boost(player, world_ref, is_in_boss_room);

            player.last_position = player.position;
            player.flush_packets();
        }
        for chunk in &mut server.world.chunk_grid.chunks {
            chunk.packet_buffer = PacketBuffer::new();
        }
        // println!("time elapsed {:?}", start.elapsed());
    }
}


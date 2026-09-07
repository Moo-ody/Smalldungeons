use crate::dungeon::door::DoorType;
use crate::dungeon::dungeon::Dungeon;
use crate::dungeon::room::room_data::RoomData;
use crate::net::internal_packets::{MainThreadMessage, NetworkThreadMessage};
use crate::net::packets::packet::ProcessPacket;
use crate::net::packets::packet_serialize::PacketSerializable;
use crate::net::protocol::play::clientbound::{AddEffect, CustomPayload, EntityProperties, JoinGame, PlayerAbilities, PlayerListHeaderFooter, PositionLook};
use crate::net::var_int::VarInt;
use crate::server::block::blocks::Blocks;
use crate::server::items::Item;
use crate::server::player::attribute::{Attribute, AttributeMap, AttributeModifier};
use crate::server::player::inventory::ItemSlot;
use crate::server::player::player::Player;
use crate::server::utils::player_list::footer::footer;
use crate::server::utils::player_list::header::header;
use crate::server::utils::tasks::Task;
use crate::server::world;
use crate::server::world::World;
use crate::server::entity::entity::EntityId;
use crate::server::utils::dvec3::DVec3;
use crate::utils::hasher::deterministic_hasher::DeterministicHashMap;
use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

pub struct Server {
    pub network_tx: UnboundedSender<NetworkThreadMessage>,
    /// the main world for this impl.
    /// in minecraft a server can have more than 1 world.
    /// however we don't really need that, so for now only 1 main world will be supported
    pub world: World,
    pub dungeon: Dungeon,

    pub tasks: Vec<Task>,
    // im not sure about having players in server directly.

    /// Static room/door template data, parsed once at boot from the bundled JSON/txt data
    /// files and reused for every dungeon built afterward (including on-demand rebuilds via
    /// `dungeon_switch::switch_dungeon`) - re-parsing hundreds of room files on every switch
    /// would undercut the "fast/seamless" requirement for no benefit, since none of this data
    /// is ever mutated per-run.
    pub room_data_storage: DeterministicHashMap<usize, RoomData>,
    pub door_type_blocks: HashMap<DoorType, Vec<Vec<Blocks>>>,

    /// Set once at startup from the `practice` CLI launch arg (see `main.rs`). Gates whether
    /// `/practice` and `/rs` (`dungeon::practice`) are allowed to run at all - practice mode is
    /// a distinct server launch mode, not an in-game toggle available on a normal dungeon run.
    pub practice_mode: bool,
}
impl Server {
    pub fn initialize_with_dungeon(
        network_tx: UnboundedSender<NetworkThreadMessage>,
        dungeon: Dungeon,
        room_data_storage: DeterministicHashMap<usize, RoomData>,
        door_type_blocks: HashMap<DoorType, Vec<Vec<Blocks>>>,
    ) -> Server {
        Server {
            network_tx,
            world: World::new(),
            dungeon,
            tasks: Vec::new(),
            room_data_storage,
            door_type_blocks,
            practice_mode: false,
        }
    }

    pub fn schedule(&mut self, run_in: u32, task: impl FnOnce(&mut Self) + 'static) {
        self.tasks.push(Task::new(run_in, task));
    }

    
    /// Schedule Bonzo projectile velocity application after delay
    pub fn schedule_bonzo_velocity(&mut self, projectile_id: EntityId, direction: DVec3, delay_ticks: u32) {
        self.schedule(delay_ticks, move |server| {
            use crate::server::items::bonzo_projectile::BonzoProjectileImpl;
            
            if let Some((_, entity_impl)) = server.world.entities.get_mut(&projectile_id) {
                // We need to replace the entity impl with a new one that has velocity
                // This is a bit hacky but works for our use case
                let new_impl = BonzoProjectileImpl::new(0, direction, 20.0); // 20 blocks/sec speed
                *entity_impl = Box::new(new_impl);
                
                // Also send velocity packet to all players
                for player in server.world.players.values_mut() {
                    use crate::net::protocol::play::clientbound::EntityVelocity;
                    use crate::net::var_int::VarInt;
                    
                    let _ = player.write_packet(&EntityVelocity {
                        entity_id: projectile_id,
                        velocity_x: direction.x * 20.0 / 20.0,
                        velocity_y: direction.y * 20.0 / 20.0,
                        velocity_z: direction.z * 20.0 / 20.0,
                    });
                }
            }
        });
    }



    pub fn spawn_ender_pearl(&mut self, player: &mut Player, velocity: crate::server::utils::dvec3::DVec3) -> anyhow::Result<()> {
        use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
        use crate::server::items::ender_pearl::PearlEntityImpl;

        let eye_height = 1.62; // player eye height in blocks
        let eye_pos = crate::server::utils::dvec3::DVec3::new(
            player.position.x,
            player.position.y + eye_height,
            player.position.z,
        );

        // Convert yaw/pitch (degrees) to a forward direction vector
        let yaw_rad = (player.yaw as f64).to_radians();
        let pitch_rad = (player.pitch as f64).to_radians();
        let dir = crate::server::utils::dvec3::DVec3::new(
            -pitch_rad.cos() * yaw_rad.sin(),
            -pitch_rad.sin(),
            pitch_rad.cos() * yaw_rad.cos(),
        );
        let dir = dir.normalize();

        let spawn_pos = crate::server::utils::dvec3::DVec3::new(
            eye_pos.x + dir.x * 0.2,
            eye_pos.y + dir.y * 0.2,
            eye_pos.z + dir.z * 0.2,
        ); // slight offset in front of player

        self.world.spawn_entity(
            spawn_pos,
            EntityMetadata::new(EntityVariant::EnderPearl),
            PearlEntityImpl::new(player.client_id, velocity),
        )?;

        Ok(())
    }

    pub fn process_event(&mut self, event: MainThreadMessage) -> Result<()> {
        match event {
            MainThreadMessage::NewPlayer { client_id, profile } => {
                println!("added player with id {client_id}");

                let spawn_pos = self.world.spawn_point;

                let mut player = Player::new(
                    self,
                    client_id,
                    profile,
                    spawn_pos,
                    self.world.spawn_yaw,
                    self.world.spawn_pitch,
                );
                println!("player entity id: {}", player.entity_id);

                player.write_packet(&JoinGame {
                    entity_id: player.entity_id,
                    gamemode: 0,
                    dimension: 0,
                    difficulty: 0,
                    max_players: 0,
                    level_type: "",
                    reduced_debug_info: false,
                });
                player.write_packet(&PositionLook {
                    x: player.position.x,
                    y: player.position.y,
                    z: player.position.z,
                    yaw: player.yaw,
                    pitch: player.pitch,
                    flags: 0,
                });

                // Full chunk+entity resync around the player's current position - also reused
                // as-is by `dungeon_switch::switch_dungeon` to resync every connected player
                // into the freshly rebuilt dungeon.
                sync_player_view(&mut self.world, &mut player);

                // Tic Tac Toe's 3 map designs are sent once at room setup, which at boot happens
                // with no players connected yet - resend to every actual joiner here so a filled
                // map item (e.g. the bot's opening move) doesn't render as an unknown/blank map.
                crate::dungeon::room::tic_tac_toe::send_map_definitions_to(&mut player);


                
                player.sidebar.write_init_packets(&mut player.packet_buffer);

                // Full tab-list snapshot (all 80 lines, including the "Dungeon: Catacombs"
                // line at index 0) - without this, a newly-joining player only ever gets tab
                // list content via `get_packet()`'s delta, which only includes lines changed
                // since the last drain. `set_line(0, ...)` runs once at server startup, so
                // that delta only ever reaches whichever players happened to already be
                // connected on the very first tick - anyone joining afterward (i.e. virtually
                // always) never received this at all. Client mods that detect "is this a
                // dungeon" by scanning tab-list entries for "Area:"/"Dungeon:"-prefixed text
                // (e.g. OdinClient's `LocationUtils`) depend on this line actually arriving.
                player.write_packet(&self.world.player_info.new_packet());

                player.write_packet(&PlayerListHeaderFooter {
                    header: header(),
                    footer: footer(),
                });
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

                // let mut map = DungeonMap::new();
                //
                // for i in 1..36 {
                //     for j in 0..4 {
                //         map.fill_px(i * 3, j * 3, 3, 3, ((i * 4) + j) as u8)
                //     }
                // }
                //
                // player.write_packet(&Maps {
                //     id: 1,
                //     scale: 0,
                //     columns: 128,
                //     rows: 128,
                //     x: 0,car
                //     z: 0,
                //     map_data: map.map_data.to_vec(),
                // });

                player.inventory.set_slot(ItemSlot::Filled(Item::AspectOfTheVoid, 1), 37);
                player.inventory.set_slot(ItemSlot::Filled(Item::DiamondPickaxe, 1), 38);
                player.inventory.set_slot(ItemSlot::Filled(Item::SpiritSceptre, 1), 39);
                player.inventory.set_slot(ItemSlot::Filled(Item::EnderPearl, 16), 43);
                player.inventory.set_slot(ItemSlot::Filled(Item::MagicalMap, 1), 44);
                player.inventory.set_slot(ItemSlot::Filled(Item::Hyperion, 1), 36);
                player.inventory.set_slot(ItemSlot::Filled(Item::TacticalInsertion, 16), 41);
                player.inventory.set_slot(ItemSlot::Filled(Item::SuperboomTNT, 64), 40);
                player.inventory.set_slot(ItemSlot::Filled(Item::GoldenAxe, 1), 13);
                player.inventory.set_slot(ItemSlot::Filled(Item::Terminator, 1), 42);
                player.inventory.set_slot(ItemSlot::Filled(Item::BonzoStaff, 1), 14);
                player.inventory.set_slot(ItemSlot::Filled(Item::JerryChineGun, 1), 15);
                player.inventory.set_slot(ItemSlot::Filled(Item::VanillaChest, 64), 16);
                // Boots armor slot (5=helmet, 6=chestplate, 7=leggings, 8=boots) - was previously
                // only given via the `/depthstrider` testing command; now on by default so water
                // in the dungeon doesn't slow players down without them having to ask for it.
                player.inventory.set_slot(ItemSlot::Filled(Item::DepthStriderBoots, 1), 8);

                player.sync_inventory();

                let playerspeed: f32 = 500.0 * 0.001;

                let mut attributes = AttributeMap::new();
                attributes.insert(Attribute::MovementSpeed, playerspeed as f64);
                attributes.add_modify(Attribute::MovementSpeed, AttributeModifier {
                    id: Uuid::parse_str("662a6b8d-da3e-4c1c-8813-96ea6097278d")?,
                    amount: 0.3, // this is always 0.3 for hypixels speed stuff
                    operation: 2,
                });

                player.write_packet(&EntityProperties {
                    entity_id: VarInt(player.entity_id),
                    properties: attributes, // this gets sent every time you sprint for some reason
                });

                player.write_packet(&PlayerAbilities {
                    invulnerable: false,
                    flying: false,
                    allow_flying: false,
                    creative_mode: false,
                    fly_speed: 0.0,
                    walk_speed: playerspeed,
                });
                
                 // Send Hypixel brand to make mods like Skytils detect this as Hypixel
                 // Fixed: Properly format as Minecraft string (varint length + bytes) with padding
                 // to prevent IndexOutOfBoundsException in mods that over-read
                 use crate::net::var_int::write_var_int;
                 
                 let mut data: Vec<u8> = Vec::new();
                 
                 // Minecraft String "Hypixel Network"
                 let brand = b"Hypixel Network";
                 write_var_int(&mut data, brand.len() as i32);
                 data.extend_from_slice(brand);
                 
                 // Pad the payload to avoid OOB reads from mods (Skytils/Essential/Patcher)
                 data.extend_from_slice(&[0u8; 4]);
                 
                 player.write_packet(&CustomPayload {
                     channel: "MC|Brand".into(),
                     data: &data,
                 });

                 // Hypixel Mod API: send Hello first so client sets onHypixel = true
                 let hello_data = crate::server::hypixel_mod_api::build_hello_payload();
                 player.write_packet(&CustomPayload {
                     channel: crate::server::hypixel_mod_api::HYPIXEL_HELLO_CHANNEL.into(),
                     data: &hello_data,
                 });
                 // Then location so Skytils (SBInfo, LocationChangeEvent), SBA, etc. see server/lobby/mode.
                 // Skytils uses mode for SkyblockIsland.byMode (e.g. "dungeon", "hub", "dynamic").
                 let location_data = crate::server::hypixel_mod_api::build_location_payload(
                     "mini1D",           // serverName (SBInfo.server)
                     Some("SKYBLOCK"),   // serverType (SBInfo.serverType, sets Utils.inSkyblock)
                     Some("lobby"),      // lobbyName
                     Some("dungeon"),    // mode -> Skytils SkyblockIsland.current (Dungeon)
                     None,               // map
                 );
                 player.write_packet(&CustomPayload {
                     channel: crate::server::hypixel_mod_api::HYPIXEL_LOCATION_CHANNEL.into(),
                     data: &location_data,
                 });
                
                player.flush_packets();

                self.world.players.insert(client_id, player);
            },
            MainThreadMessage::ClientDisconnected { client_id } => {
                self.world.players.remove(&client_id);
                println!("Client {} disconnected", client_id);
            },
            MainThreadMessage::PacketReceived { client_id, packet } => {
                let player = self.world.players.get_mut(&client_id).context(format!("Player not found for id {client_id}"))?;
                packet.process_with_player(player);
            },
            MainThreadMessage::Abort { reason } => {
                panic!("Network called for shutdown: {}", reason);
            },
        }
        Ok(())
    }
}

/// Chunk radius sent synchronously by `sync_player_view` - the rest of the view distance is
/// queued into `player.pending_chunk_sync` and trickled in a few at a time by
/// `drain_pending_chunk_sync` instead. See that field's doc comment for why: this radius still
/// covers well more than a player's immediate surroundings (where an item they use the instant
/// they spawn in, like an arrow, would land), so the one part of the view that has to be usable
/// *immediately* is never deferred.
const IMMEDIATE_SYNC_RADIUS: i32 = 2;

/// Sends `player` a full, authoritative snapshot of everything currently in view of their
/// position: fresh chunk data (`new = true`, so it fully replaces whatever the client
/// previously had for that chunk - no stray blocks can survive this) plus spawn/equipment
/// packets for every entity in those chunks. Used both for a brand-new player's initial join
/// and by `dungeon_switch::switch_dungeon` to resync every connected player into a freshly
/// rebuilt dungeon.
///
/// Only the immediate `IMMEDIATE_SYNC_RADIUS` chunks around `player` are sent synchronously here
/// - the remaining ring out to the full view distance is queued into `player.pending_chunk_sync`
/// and trickled a handful per tick by `drain_pending_chunk_sync` (called from `main.rs`'s tick
/// loop) instead of all landing in one burst. Sending the *entire* view distance at once (up to
/// 169 chunks at `VIEW_DISTANCE=6`) used to be genuinely correct as far as this server's own
/// packet delivery goes, but overwhelms the vanilla 1.8 client's own chunk mesh-building queue -
/// any entity spawned in a chunk whose render mesh hasn't been built yet renders as invisible
/// until a full client-side reload (F3+A) forces every chunk to rebuild at once. That's exactly
/// "sometimes entities need F3+A to show, like arrows, if I join and shoot the bow": a fired
/// arrow's spawn packet landing in the same tick as (or right after) a hundred-plus still-
/// unmeshed chunks. Splitting the burst doesn't change what eventually gets sent, just how
/// quickly - the full view distance still fills in within well under a second either way.
pub fn sync_player_view(world: &mut World, player: &mut Player) {
    let chunk_x = (player.position.x.floor() as i32) >> 4;
    let chunk_z = (player.position.z.floor() as i32) >> 4;
    let view_distance = world::VIEW_DISTANCE as i32 + 1;

    let mut pending: Vec<(i32, i32)> = Vec::new();

    world.chunk_grid.for_each_in_view(
        chunk_x,
        chunk_z,
        view_distance,
        |chunk, x, z| {
            if (x - chunk_x).abs() > IMMEDIATE_SYNC_RADIUS || (z - chunk_z).abs() > IMMEDIATE_SYNC_RADIUS {
                pending.push((x, z));
                return;
            }

            player.write_packet(&chunk.get_chunk_data(x, z, true));

            for entity_id in chunk.entities.iter_mut() {
                // A stale ID here (chunk-membership desync) should never crash this - skip it
                // rather than unwrap.
                let Some((entity, entity_impl)) = world.entities.get_mut(&entity_id) else { continue };
                // Player-model entities (e.g. Mort) need their tab-list entry before
                // SpawnPlayer or they render invisible - see `write_entity_spawn`.
                crate::server::world::write_entity_spawn(entity, entity_impl.as_mut(), &mut player.packet_buffer);

                // Resync equipment if this entity has equipment
                if let Some(equipment) = world.entity_equipment.get(&*entity_id) {
                    use crate::server::entity::spawn_equipped::send_equipment_packets;
                    send_equipment_packets(&mut player.packet_buffer, *entity_id, equipment);
                }
            }
        }
    );

    // Nearest-first, so the trickle fills in outward from the player rather than in raw scan
    // order - the closer ring is more likely to matter (about to walk into view) than the far
    // edge of the view distance.
    pending.sort_by_key(|&(x, z)| (x - chunk_x).abs().max((z - chunk_z).abs()));
    player.pending_chunk_sync = pending;
}

/// How many queued `pending_chunk_sync` chunks to actually send each tick - see that field's and
/// `sync_player_view`'s doc comments for why this exists. Small enough that the client's own
/// chunk mesh-building never falls far behind the chunks it's being told about, but the full
/// view distance still fills in within about a second (169 chunks / 8 per tick ≈ 21 ticks).
const PENDING_CHUNKS_PER_TICK: usize = 8;

/// Sends the next `PENDING_CHUNKS_PER_TICK` chunks (if any) queued by `sync_player_view` for
/// `player` - a no-op once the queue is empty, i.e. every tick after the first second or so
/// following a join or dungeon resync.
pub fn drain_pending_chunk_sync(player: &mut Player) {
    if player.pending_chunk_sync.is_empty() {
        return;
    }
    let take = player.pending_chunk_sync.len().min(PENDING_CHUNKS_PER_TICK);
    let batch: Vec<(i32, i32)> = player.pending_chunk_sync.drain(..take).collect();
    send_chunk_batch(player, batch);
}

/// Sends *every* chunk still queued in `player.pending_chunk_sync` right now, instead of
/// trickling `PENDING_CHUNKS_PER_TICK` per tick - use this immediately before teleporting a
/// player somewhere they might act on the destination instantly (`/practice`'s teleport is the
/// motivating case). `/practice` moves a player by directly overwriting `position`/
/// `last_position` together (see `practice::teleport_player`), which deliberately skips the
/// normal "you moved, resync newly-visible chunks" path in `main.rs`'s tick loop (that path
/// diffs against `last_position`, and there's no diff when the two already match) - so a
/// teleport is the *one* case that doesn't eventually self-correct via ordinary movement. Without
/// this, a `/practice` run soon enough after joining that some of the destination room's chunks
/// were still sitting in the staggered queue (see `sync_player_view`'s doc comment for why that
/// queue exists at all) would land the player in a room real block data hasn't fully arrived for
/// yet - invisible to the player only briefly (the queue drains within about a second either
/// way), but long enough that a mod doing a *one-time* read of the room's blocks on arrival (real
/// Odin's `WorldScan` room-identification is exactly this: a one-shot block-hash scan on room
/// entry, no retry if it reads incomplete data) can permanently misidentify the room for that
/// visit. Larger rooms - Blaze's tall shaft being the most extreme in the whole dungeon - are the
/// most likely to have some of their footprint still queued this soon after a join.
pub fn flush_all_pending_chunk_sync(player: &mut Player) {
    if player.pending_chunk_sync.is_empty() {
        return;
    }
    let batch: Vec<(i32, i32)> = player.pending_chunk_sync.drain(..).collect();
    send_chunk_batch(player, batch);
}

fn send_chunk_batch(player: &mut Player, batch: Vec<(i32, i32)>) {
    let world = player.world_mut();
    for (x, z) in batch {
        let Some(chunk) = world.chunk_grid.get_chunk_mut(x, z) else { continue };
        player.write_packet(&chunk.get_chunk_data(x, z, true));

        // A stale ID here (chunk-membership desync) should never crash this - collect only the
        // still-alive ones first, same as `main.rs`'s own `ChunkDiff::New` handling this replaces
        // for a moved/teleported player - and prune the rest from the chunk's own list below so
        // they don't linger forever (harmless if another player's batch already did it first).
        let valid_entity_ids: Vec<_> = chunk.entities.iter()
            .filter(|&&entity_id| world.entities.contains_key(&entity_id))
            .copied()
            .collect();

        for &entity_id in &valid_entity_ids {
            let Some((entity, entity_impl)) = world.entities.get_mut(&entity_id) else { continue };
            crate::server::world::write_entity_spawn(entity, entity_impl.as_mut(), &mut player.packet_buffer);

            if let Some(equipment) = world.entity_equipment.get(&entity_id) {
                use crate::server::entity::spawn_equipped::send_equipment_packets;
                send_equipment_packets(&mut player.packet_buffer, entity_id, equipment);
            }
        }

        let Some(chunk) = world.chunk_grid.get_chunk_mut(x, z) else { continue };
        chunk.entities.clear();
        chunk.entities.extend(valid_entity_ids);
    }
}
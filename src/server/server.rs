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
                 
                 // Log the custom payload before sending
                 let hex_dump: String = data.iter()
                     .take(32) // Show first 32 bytes
                     .map(|b| format!("{:02x}", b))
                     .collect::<Vec<_>>()
                     .join(" ");
                 let hex_suffix = if data.len() > 32 { "..." } else { "" };
                 println!(
                     "[RC DEBUG] sending custom payload: channel='MC|Brand', len={}, hex={}{}",
                     data.len(),
                     hex_dump,
                     hex_suffix
                 );
                 
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

/// Sends `player` a full, authoritative snapshot of everything currently in view of their
/// position: fresh chunk data (`new = true`, so it fully replaces whatever the client
/// previously had for that chunk - no stray blocks can survive this) plus spawn/equipment
/// packets for every entity in those chunks. Used both for a brand-new player's initial join
/// and by `dungeon_switch::switch_dungeon` to resync every connected player into a freshly
/// rebuilt dungeon in one atomic burst.
pub fn sync_player_view(world: &mut World, player: &mut Player) {
    let chunk_x = (player.position.x.floor() as i32) >> 4;
    let chunk_z = (player.position.z.floor() as i32) >> 4;
    let view_distance = world::VIEW_DISTANCE as i32 + 1;

    world.chunk_grid.for_each_in_view(
        chunk_x,
        chunk_z,
        view_distance,
        |chunk, x, z| {
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
}
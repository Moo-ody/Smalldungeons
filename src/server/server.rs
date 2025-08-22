use crate::dungeon::dungeon::Dungeon;
use crate::net::internal_packets::{MainThreadMessage, NetworkThreadMessage};
use crate::net::packets::packet::ProcessPacket;
use crate::net::protocol::play::clientbound::{AddEffect, EntityProperties, JoinGame, PlayerAbilities, PlayerListHeaderFooter, PositionLook};
use crate::net::var_int::VarInt;
use crate::server::items::Item;
use crate::server::player::attribute::{Attribute, AttributeMap, AttributeModifier};
use crate::server::player::inventory::ItemSlot;
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::player_list::footer::footer;
use crate::server::utils::player_list::header::header;
use crate::server::utils::tasks::{Task, TaskType};
use crate::server::world;
use crate::server::world::World;
use anyhow::{Context, Result};
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
}
impl Server {
    pub fn initialize_with_dungeon(
        network_tx: UnboundedSender<NetworkThreadMessage>,
        dungeon: Dungeon,
    ) -> Server {
        Server {
            network_tx,
            world: World::new(),
            dungeon,
            tasks: Vec::new(),
        }
    }

    pub fn schedule(&mut self, run_in: u32, task: impl FnOnce(&mut Self) + 'static) {
        self.tasks.push(Task::new(run_in, task));
    }

    pub fn process_event(&mut self, event: MainThreadMessage) -> Result<()> {
        match event {
            MainThreadMessage::NewPlayer { client_id, profile } => {
                println!("added player with id {client_id}");

                let spawn_point = DVec3 {
                    x: 20.0,
                    y: 69.0,
                    z: 20.0,
                };

                let mut player = Player::new(
                    self,
                    client_id,
                    profile,
                    spawn_point,
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

                let chunk_x = (player.position.x.floor() as i32) >> 4;
                let chunk_z = (player.position.z.floor() as i32) >> 4;

                let view_distance = world::VIEW_DISTANCE as i32;
                for x in (chunk_x - view_distance)..(chunk_x + view_distance) {
                    for z in (chunk_z - view_distance)..(chunk_z + view_distance) {
                        if let Some(chunk) = self.world.chunk_grid.get_chunk(x, z) {
                            player.write_packet(&chunk.get_chunk_data(true))
                        }
                    }
                }
                player.sidebar.write_init_packets(&mut player.packet_buffer);

                // player.send_packet(self.world.player_info.new_packet())?;

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

                player.inventory.set_slot(ItemSlot::Filled(Item::AspectOfTheVoid), 36);
                player.inventory.set_slot(ItemSlot::Filled(Item::DiamondPickaxe), 37);
                player.inventory.set_slot(ItemSlot::Filled(Item::SkyblockMenu), 44);
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
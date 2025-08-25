use crate::dungeon::dungeon::Dungeon;
use crate::net::internal_packets::{MainThreadMessage, NetworkThreadMessage};
use crate::net::packets::client_bound::chunk_data::ChunkData;
use crate::net::packets::client_bound::entity::entity_effect::{Effects, EntityEffect};
use crate::net::packets::client_bound::entity::entity_properties::EntityProperties;
use crate::net::packets::client_bound::join_game::JoinGame;
use crate::net::packets::client_bound::player_list_header_footer::PlayerListHeaderFooter;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::ServerBoundPacket;
use crate::server::items::Item;
use crate::server::player::attribute::{Attribute, AttributeMap};
use crate::server::player::inventory::ItemSlot;
use crate::server::player::player::{GameProfile, Player};
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::player_list::footer::footer;
use crate::server::utils::player_list::header::header;
use crate::server::world::World;
use anyhow::{anyhow, Result};
use tokio::sync::mpsc::UnboundedSender;

pub struct Server {
    pub network_tx: UnboundedSender<NetworkThreadMessage>,
    /// the main world for this impl.
    /// in minecraft a server can have more than 1 world.
    /// however we don't really need that, so for now only 1 main world will be supported
    pub world: World,
    pub dungeon: Dungeon,
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
        }
    }

    pub fn process_event(&mut self, event: MainThreadMessage) -> Result<()> {
        match event {
            MainThreadMessage::NewPlayer { client_id, username } => {
                println!("added player with id {client_id}");

                let spawn_pos = self.world.spawn_point.as_vec3f();

                let mut player = Player::new(
                    self,
                    client_id,
                    // todo, add uuid and other stuff
                    GameProfile {
                        username
                    },
                    spawn_pos,
                );
                println!("player entity id: {}", player.entity_id);

                player.send_packet(JoinGame::from_player(&player))?;
                player.send_packet(PositionLook::from_player(&player))?;

                for chunk in self.world.chunk_grid.chunks.iter() {
                    player.send_packet(ChunkData::from_chunk(chunk, true))?;
                }

                for packet in player.sidebar.packets_to_init() {
                    packet.send_packet(client_id, &self.network_tx)?;
                }

                // for entity in self.world.entities.values_mut() {
                //     if entity.entity_id == player.entity_id {
                //         continue
                //     }
                //     println!("entity_id: {}, name: {:?}", entity.entity_id, entity.entity_type);
                //     player.observe_entity(entity, &self.network_tx)?
                // }

                player.send_packet(self.world.player_info.new_packet())?;

                player.send_packet(PlayerListHeaderFooter {
                    header: header(),
                    footer: footer(),
                })?;
                player.send_packet(EntityEffect {
                    entity_id: player.entity_id,
                    effect: Effects::Haste,
                    amplifier: 2,
                    duration: 200,
                    hide_particles: true,
                })?;
                player.send_packet(EntityEffect {
                    entity_id: player.entity_id,
                    effect: Effects::NightVision,
                    amplifier: 0,
                    duration: 400,
                    hide_particles: true,
                })?;

                player.inventory.set_slot(ItemSlot::Filled(Item::AspectOfTheVoid), 36);
                player.inventory.set_slot(ItemSlot::Filled(Item::DiamondPickaxe), 37);
                player.inventory.set_slot(ItemSlot::Filled(Item::SkyblockMenu), 44);
                player.sync_inventory()?;

                let mut attributes = AttributeMap::new();
                attributes.insert(Attribute::MovementSpeed, 0.4);

                player.send_packet(EntityProperties {
                    entity_id: player.entity_id,
                    properties: attributes,
                })?;

                // let entity = self.world.spawn_entity(spawn_point, Zombie, None)?;

                self.world.players.insert(client_id, player);
                //
                // let mut buf = Vec::new();
                // "hypixel".write(&mut buf);
                //
                // CustomPayload {
                //     channel: "MC|Brand".into(),
                //     data: buf,
                // }.send_packet(client_id, &self.network_tx)?;
            },
            MainThreadMessage::ClientDisconnected { client_id } => {
                self.world.players.remove(&client_id);
                println!("Client {} disconnected", client_id);
            },
            MainThreadMessage::PacketReceived { client_id, packet } => {
                let player = self.world.players.get_mut(&client_id).ok_or_else(|| anyhow!("Player not found for id {client_id}"))?;
                packet.main_process(&mut player.server_mut().world, player)?;
            }
        }
        Ok(())
    }
}
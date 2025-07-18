use crate::dungeon::Dungeon;
use crate::net::internal_packets::{MainThreadMessage, NetworkThreadMessage};
use crate::net::packets::client_bound::chunk_data::ChunkData;
use crate::net::packets::client_bound::entity::entity_effect::{EntityEffect, HASTEID, NIGHTVISIONID};
use crate::net::packets::client_bound::entity::entity_properties::EntityProperties;
use crate::net::packets::client_bound::join_game::JoinGame;
use crate::net::packets::client_bound::player_list_header_footer::PlayerListHeaderFooter;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::{SendPacket, ServerBoundPacket};
use crate::server::entity::attributes::Attribute;
use crate::server::entity::attributes::AttributeTypes::MovementSpeed;
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_type::EntityType;
use crate::server::items::Item;
use crate::server::player::inventory::ItemSlot;
use crate::server::player::{ClientId, Player};
use crate::server::utils::player_list::footer::footer;
use crate::server::utils::player_list::header::header;
use crate::server::utils::vec3d::DVec3;
use crate::server::world::World;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;

pub struct Server {
    pub network_tx: UnboundedSender<NetworkThreadMessage>,
    /// the main world for this impl.
    /// in minecraft a server can have more than 1 world.
    /// however we don't really need that, so for now only 1 main world will be supported
    pub world: World,
    pub dungeon: Dungeon,
    // im not sure about having players in server directly.
    pub players: HashMap<ClientId, Player>,
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
            players: HashMap::new(),
        }
    }

    pub fn process_event(&mut self, event: MainThreadMessage) -> Result<()> {
        match event {
            MainThreadMessage::NewPlayer { client_id, username } => {
                println!("added player with id {client_id}");

                let spawn_point = DVec3 {
                    x: 20.0,
                    y: 69.0,
                    z: 20.0,
                };

                let player_entity = Entity::create_at(EntityType::Player, spawn_point, self.world.new_entity_id());
                println!("player entity id: {}", player_entity.entity_id);
                let mut player = Player::new(
                    self,
                    username,
                    client_id,
                    player_entity.entity_id
                );

                JoinGame::from_entity(&player_entity).send_packet(client_id, &self.network_tx)?;
                PositionLook::from_entity(&player_entity).send_packet(client_id, &self.network_tx)?;

                self.world.entities.insert(player_entity.entity_id, player_entity);

                for chunk in self.world.chunk_grid.chunks.iter() {
                    ChunkData::from_chunk(chunk, true).send_packet(client_id, &self.network_tx)?;
                }

                for entity in self.world.entities.values_mut() {
                    if entity.entity_id == player.entity_id { continue }
                    println!("entity_id: {}, name: {:?}", entity.entity_id, entity.entity_type);
                    player.observe_entity(entity, &self.network_tx)?
                }

                PlayerListHeaderFooter {
                    header: header(),
                    footer: footer(),
                }.send_packet(player.client_id, &self.network_tx)?;

                let scoreboard = player.scoreboard.packets_to_init();
                for packet in scoreboard {
                    packet.send_packet(client_id, &self.network_tx)?;
                }

                self.world.player_info.new_packet().send_packet(client_id, &self.network_tx)?;

                for packet in player.scoreboard.packets_to_init() {
                    packet.send_packet(client_id, &self.network_tx)?;
                }

                EntityEffect {
                    entity_id: player.entity_id,
                    effect_id: HASTEID,
                    amplifier: 2,
                    duration: 200,
                    hide_particles: true,
                }.send_packet(player.client_id, &self.network_tx)?;

                EntityEffect {
                    entity_id: player.entity_id,
                    effect_id: NIGHTVISIONID,
                    amplifier: 0,
                    duration: 400,
                    hide_particles: true,
                }.send_packet(player.client_id, &self.network_tx)?;

                player.inventory.set_slot(ItemSlot::Filled(Item::AspectOfTheVoid), 36);
                player.inventory.set_slot(ItemSlot::Filled(Item::DiamondPickaxe), 37);
                player.inventory.set_slot(ItemSlot::Filled(Item::SkyblockMenu), 44);

                let mut properties = HashMap::new();
                properties.insert(MovementSpeed, Attribute::new(0.4));

                EntityProperties {
                    entity_id: player.entity_id,
                    properties,
                }.send_packet(player.client_id, &self.network_tx)?;

                player.sync_inventory()?;
                self.players.insert(client_id, player);
            },
            MainThreadMessage::ClientDisconnected { client_id } => {
                if let Some(player) = self.players.remove(&client_id) {
                    for entity_id in player.observed_entities {
                        if let Some(entity) = self.world.entities.get_mut(&entity_id) {
                            // doesnt call stop_observing_entity because player is borrowed to get its observed entities ids.
                            // and the player object itself should be destroyed sometime here.
                            entity.observing_players.remove(&client_id);
                        }
                    }

                    self.world.entities.remove(&player.entity_id);
                }
                println!("Client {} disconnected", client_id);
            },
            MainThreadMessage::PacketReceived { client_id, packet } => {
                packet.main_process(&mut self.world, self.players.get_mut(&client_id).ok_or_else(|| anyhow!("Player not found for id {client_id}"))?)?;
            }
        }
        Ok(())
    }
}
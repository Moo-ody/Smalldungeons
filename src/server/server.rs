use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::chunk_data::ChunkData;
use crate::net::packets::client_bound::entity::entity_effect::{EntityEffect, HASTEID};
use crate::net::packets::client_bound::join_game::JoinGame;
use crate::net::packets::client_bound::player_list_header_footer::PlayerListHeaderFooter;
use crate::net::packets::client_bound::player_list_item::PlayerListItem;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::{SendPacket, ServerBoundPacket};
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_type::EntityType;
use crate::server::items::Item;
use crate::server::player::inventory::ItemSlot;
use crate::server::player::{ClientId, Player};
use crate::server::utils::player_list::footer::footer;
use crate::server::utils::player_list::header::header;
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;

pub struct Server {
    pub network_tx: UnboundedSender<NetworkMessage>,
    /// the main world for this impl.
    /// in minecraft a server can have more than 1 world.
    /// however we don't really need that, so for now only 1 main world will be supported
    pub world: World,
    // im not sure about having players in server directly.
    pub players: HashMap<ClientId, Player>,
}
impl Server {
    pub fn initialize(network_tx: UnboundedSender<NetworkMessage>) -> Server {
        Server {
            world: World::new(),
            network_tx,
            players: HashMap::new()
        }
    }

    pub fn process_event(&mut self, event: ClientEvent) -> Result<()> {
        match event {
            ClientEvent::NewPlayer { client_id } => {
                println!("added player with id {client_id}");

                let spawn_point = Vec3f {
                    x: 3.0,
                    y: 1.0,
                    z: 3.0,
                };

                let player_entity = Entity::create_at(EntityType::Player, spawn_point, self.world.new_entity_id());
                let mut player = Player::new(self, client_id, player_entity.entity_id);

                JoinGame::from_entity(&player_entity).send_packet(client_id, &self.network_tx)?;
                PositionLook::from_entity(&player_entity).send_packet(client_id, &self.network_tx)?;

                self.world.entities.insert(player_entity.entity_id, player_entity);

                for chunk in self.world.chunk_grid.chunks.iter() {
                    ChunkData::from_chunk(chunk, true).send_packet(client_id, &self.network_tx)?;
                }

                for entity in self.world.entities.values_mut() {
                    if entity.entity_id == player.entity_id { continue }
                    player.observe_entity(entity, &self.network_tx)?
                }

                PlayerListItem::init_packet(self.world.player_info.tab_list()).send_packet(client_id, &self.network_tx)?;

                PlayerListHeaderFooter {
                    header: header(),
                    footer: footer(),
                }.send_packet(player.client_id, &self.network_tx)?;

                player.scoreboard.header_packet().send_packet(player.client_id, &self.network_tx)?;

                for packet in player.scoreboard.get_packets() {
                    packet.send_packet(player.client_id, &self.network_tx)?;
                }

                player.scoreboard.display_packet().send_packet(player.client_id, &self.network_tx)?;

                player.scoreboard.add_line("roomid", "§706/14/25 §8m24§87W 730,-420");
                player.scoreboard.add_line("e1", "");
                player.scoreboard.add_line("season", "Winter 22nd");
                player.scoreboard.add_line("ctime", "§73:10pm");
                player.scoreboard.add_line("zone", " §7⏣ §cThe Catac§combs §7(F7)");
                player.scoreboard.add_line("e2", "");
                // boxes stay, red gets ✔ or ✖ depending on blood key and gray increments counter per wither key probably
                // im not sure if these are the right box symbols but well have to see
                player.scoreboard.add_line("keys", "Keys: §c■ §c✖ §8§8■ §a0x");
                player.scoreboard.add_line("etime", "Time Elapsed: §a§a00s");
                player.scoreboard.add_line("clear", "Cleared: §c0% §8§8(0)");
                player.scoreboard.add_line("s1", "          ");
                player.scoreboard.add_line("solo", "§3§lSolo");
                player.scoreboard.add_line("s2", "          ");
                player.scoreboard.add_line("footer", "§emc.hypixel.net");

                EntityEffect {
                    entity_id: player.entity_id,
                    effect_id: HASTEID,
                    amplifier: 2,
                    duration: 200,
                    hide_particles: true,
                }.send_packet(player.client_id, &self.network_tx)?;

                // EntityEffect {
                //     entity_id: player.entity_id,
                //     effect_id: NIGHTVISIONID,
                //     amplifier: 0,
                //     duration: 400,
                //     hide_particles: true,
                // }.send_packet(player.client_id, &self.network_tx)?;

                player.inventory.set_slot(ItemSlot::Filled(Item::AspectOfTheVoid), 36);
                player.inventory.set_slot(ItemSlot::Filled(Item::DiamondPickaxe), 37);
                
                player.inventory.sync(&player, &self.network_tx)?;
                self.players.insert(client_id, player);
            },
            ClientEvent::ClientDisconnected { client_id } => {
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
            ClientEvent::PacketReceived { client_id, packet  }  => {
                // println!("Packet received from client {}: {:?}", client_id, packet);
                packet.main_process(&mut self.world, self.players.get_mut(&client_id).ok_or_else(|| anyhow!("Player not found for id {client_id}"))?)?;
                // update to match if/when its needed

                // match packet {
                //     // test
                //     // ServerBoundPackets::PlayerBlockPlacement(_) => {
                //     //     if let Some(player) = &mut self.players.get_mut(&client_id) {
                //     //         player.set_position(
                //     //             &self.network_tx,
                //     //             player.entity.pos.x,
                //     //             player.entity.pos.y + 10.0,
                //     //             player.entity.pos.z,
                //     //         )?;
                //     //     };
                //     // }
                //     _ => {}
                // }
            }
        }
        Ok(())
    }
}
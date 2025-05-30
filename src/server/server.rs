use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::chunk_data::ChunkData;
use crate::net::packets::client_bound::join_game::JoinGame;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::client_bound::spawn_mob::SpawnMob;
use crate::net::packets::packet::{SendPacket, ServerBoundPacket};
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_type::EntityType;
use crate::server::player::Player;
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
    pub players: HashMap<u32, Player>,
}

impl Server {
    pub fn initialize(network_tx: UnboundedSender<NetworkMessage>) -> Server {
        Server {
            network_tx,
            world: World::new(),
            players: HashMap::new()
        }
    }

    pub fn process_event(&mut self, event: ClientEvent) -> Result<()> {
        match event {
            ClientEvent::NewPlayer { client_id } => {
                let player_entity = Entity::create_at(EntityType::Player, Vec3f::new_empty(), self.world.new_entity_id());
                let player = Player::new(client_id, player_entity.entity_id);

                JoinGame::from_entity(&player_entity).send_packet(client_id, &self.network_tx)?;
                PositionLook::from_entity(&player_entity).send_packet(client_id, &self.network_tx)?;

                for chunk in self.world.chunks.iter() {
                    ChunkData::from_chunk(chunk, true).send_packet(client_id, &self.network_tx)?;
                }

                self.world.entities.insert(player_entity.entity_id, player_entity);
                self.players.insert(client_id, player);

                let zombie = Entity::create_at(EntityType::Zombie, Vec3f::new_empty(), self.world.new_entity_id());

                SpawnMob::from_entity(&zombie).send_packet(client_id, &self.network_tx)?;
                self.world.entities.insert(zombie.entity_id, zombie);

            },
            ClientEvent::ClientDisconnected { client_id } => {
                self.players.remove(&client_id);
                println!("Client {} disconnected", client_id);
            },
            ClientEvent::PacketReceived { client_id, packet  }  => {
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
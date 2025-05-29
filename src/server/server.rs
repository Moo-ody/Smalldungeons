use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::chunk_data::ChunkData;
use crate::net::packets::client_bound::join_game::JoinGame;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::SendPacket;
use crate::server::entity::entity::Entity;
use crate::server::player::Player;
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;
use anyhow::Result;
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
                let player = Player {
                    client_id,
                    entity: Entity::spawn_at(
                        Vec3f::new_empty(),
                        self.world.new_entity_id(),
                    ),
                };
                JoinGame::from_player(&player).send_packet(client_id, &self.network_tx)?;
                PositionLook::from_player(&player).send_packet(client_id, &self.network_tx)?;
                for chunk in self.world.chunks.iter() {
                    ChunkData::from_chunk(chunk, true).send_packet(client_id, &self.network_tx)?;
                }
                self.players.insert(client_id, player);
            },
            ClientEvent::ClientDisconnected { client_id } => {
                self.players.remove(&client_id);
                println!("Client {} disconnected", client_id);
            },
            ClientEvent::PacketReceived { client_id, packet  }  => {
                match packet {
                    // test
                    // ServerBoundPackets::PlayerBlockPlacement(_) => {
                    //     if let Some(player) = &mut self.players.get_mut(&client_id) {
                    //         player.set_position(
                    //             &self.network_tx,
                    //             player.entity.pos.x,
                    //             player.entity.pos.y + 10.0,
                    //             player.entity.pos.z,
                    //         )?;
                    //     };
                    // }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
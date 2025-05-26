mod net;
mod server;

use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::{chunk_data, join_game, position_look};
use crate::net::run_network::run_network_thread;
use crate::server::entity::entity_enum::Entity::PlayerEntity;
use crate::server::entity::player_entity;
use crate::server::world::World;
use anyhow::Result;
use crate::net::packets::packet_registry::ClientBoundPackets::{ChunkData, JoinGame, PositionLook};

const STATUS_RESPONSE_JSON: &str = r#"{
    "version": { "name": "1.8.9", "protocol": 47 },
    "players": { "max": 1, "online": 0 },
    "description": { "text": "broooo" }
}"#;

#[tokio::main]
async fn main() -> Result<()> {
    let (network_tx, mut network_rx) = tokio::sync::mpsc::unbounded_channel::<NetworkMessage>();
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<ClientEvent>();

    tokio::spawn(run_network_thread(network_rx, network_tx.clone(), event_tx.clone()));

    let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(50));

    let mut world = World::new();
    
    loop {
        tick_interval.tick().await;
        
        // Handle incoming events from network
        while let Ok(event) = event_rx.try_recv() {
            match event {
                ClientEvent::PacketReceived { client_id, packet } => {
                    //println!("Client {} sent {:?}", client_id, packet);

                    match packet {
                        _ => {}
                    }
                }
                ClientEvent::NewPlayer { client_id } => {
                    let player = player_entity::PlayerEntity::new();

                    JoinGame(join_game::JoinGame::from_player(&player)).send_packet(client_id, &network_tx)?;
                    PositionLook(position_look::PositionLook::from_player(&player)).send_packet(client_id, &network_tx)?;
                    ChunkData(chunk_data::ChunkData::new()).send_packet(client_id, &network_tx)?;

                    world.add_entity(PlayerEntity(player));
                }
                ClientEvent::ClientDisconnected { client_id } => {
                    //println!("Client {} disconnected", client_id);
                }
            }
        }

        // Game logic here...
    }
}
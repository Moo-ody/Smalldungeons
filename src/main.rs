mod net;
mod server;

use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::confirm_transaction::ConfirmTransaction;
use crate::net::packets::packet::SendPacket;
use crate::net::run_network::run_network_thread;
use crate::server::block::blocks::Blocks;
use crate::server::chunk::chunk_section::ChunkSection;
use crate::server::chunk::Chunk;
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_type::EntityType;
use crate::server::server::Server;
use crate::server::utils::vec3f::Vec3f;
use anyhow::Result;
use std::time::Duration;
use tokio::sync::mpsc::unbounded_channel;

const STATUS_RESPONSE_JSON: &str = r#"{
    "version": { "name": "1.8.9", "protocol": 47 },
    "players": { "max": 1, "online": 0 },
    "description": { "text": "RustClear", "color": "gold", "extra": [{ "text": " version ", "color": "gray" }, { "text": "0.1.0", "color": "green"}] }
}"#;

#[tokio::main]
async fn main() -> Result<()> {
    let (network_tx, network_rx) = unbounded_channel::<NetworkMessage>();
    let (event_tx, mut event_rx) = unbounded_channel::<ClientEvent>();

    let mut server = Server::initialize(network_tx);

    // example stone grid
    for x in 0..10 {
        for z in 0..10 {
            let mut chunk = Chunk::new(x, z);
            let mut chunk_section = ChunkSection::new();

            for x in 1..14 {
                for z in 1..14 {
                    chunk_section.set_block_at(Blocks::Stone, x, 0, z);
                }
            }

            chunk.add_section(chunk_section, 0);
            server.world.chunks.push(chunk);
        }
    }

    let spawn_pos = Vec3f {
        x: 6.0,
        y: 1.0,
        z: 6.0,
    };

    let zombie = Entity::create_at(EntityType::Zombie, spawn_pos, server.world.new_entity_id());

    //zombie.set_name("Dinnerbone");
    server.world.entities.insert(zombie.entity_id, zombie);

    let mut tick_interval = tokio::time::interval(Duration::from_millis(50));
    tokio::spawn(
        run_network_thread(
            network_rx,
            server.network_tx.clone(),
            event_tx.clone()
        )
    );

    loop {
        tick_interval.tick().await;

        while let Ok(message) = event_rx.try_recv() {
            server.process_event(message).unwrap_or_else(|err| eprintln!("Error processing event: {err}"));
        }

        for entity_id in server.world.entities.keys().cloned().collect::<Vec<_>>() {
            if let Some(entity) = server.world.entities.remove(&entity_id) {
                // this may at some point be abused to prevent getting an entities own self if it iterates over world entities so be careful if you change this
                let returned = entity.update(&mut server.world, &server.network_tx);
                server.world.entities.insert(entity_id, returned);
            }
        }

        // this needs to be changed to work with loaded chunks, tracking last sent data per player (maybe), etc.
        // also needs to add a method to only send the right entity packet given movement data based on last sent.
        // also needs to actually be in a vanilla adjacent way.
        for player in server.players.values() {
            println!("player ticked: {player:?}");
            ConfirmTransaction::new().send_packet(player.client_id, &server.network_tx)?; // should stop disconnects? keep alive logic would too probably.

            // for entity in player.tracked_entities.iter() {
            //     if let Some(entity) = server.world.entities.get_mut(entity) {
            //         EntityLookMove::from_entity(entity).send_packet(player.client_id, &server.network_tx)?;
            //         EntityHeadLook::new(entity.entity_id, entity.head_yaw).send_packet(player.client_id, &server.network_tx)?;
            //     }
            // }
        }

        // rest of functionality here
    }
}
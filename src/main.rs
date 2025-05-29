mod net;
mod server;

use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::run_network::run_network_thread;
use crate::server::block::Blocks;
use crate::server::chunk::chunk_section::ChunkSection;
use crate::server::chunk::Chunk;
use crate::server::server::Server;
use anyhow::Result;
use std::time::Duration;
use tokio::sync::mpsc::unbounded_channel;

const STATUS_RESPONSE_JSON: &str = r#"{
    "version": { "name": "1.8.9", "protocol": 47 },
    "players": { "max": 1, "online": 0 },
    "description": { "text": "broooo" }
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
            let result = server.process_event(message);
            if result.is_err() { 
                return result;
            }
        }
        
        // rest of functionality here
        
    }
}
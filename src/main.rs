mod net;
mod server;

use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::run_network::run_network_thread;
use anyhow::Result;
use tokio::sync::mpsc::unbounded_channel;
use crate::server::server::tick;

const STATUS_RESPONSE_JSON: &str = r#"{
    "version": { "name": "1.8.9", "protocol": 47 },
    "players": { "max": 1, "online": 0 },
    "description": { "text": "broooo" }
}"#;

#[tokio::main]
async fn main() -> Result<()> {
    let (network_tx, network_rx) = unbounded_channel::<NetworkMessage>();
    let (event_tx, event_rx) = unbounded_channel::<ClientEvent>();

    tokio::spawn(run_network_thread(network_rx, network_tx.clone(), event_tx.clone()));
    tick(event_rx, network_tx).await
}
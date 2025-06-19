use crate::net::client::handle_client;
use std::collections::HashMap;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::player::ClientId;

pub async fn run_network_thread(
    mut network_rx: UnboundedReceiver<NetworkMessage>,
    network_tx: UnboundedSender<NetworkMessage>,
    event_tx: UnboundedSender<ClientEvent>,
) {
    let listener = TcpListener::bind("127.0.0.1:4972").await.unwrap();
    println!("Network thread listening on 127.0.0.1:4972");

    let mut clients: HashMap<ClientId, UnboundedSender<Vec<u8>>> = HashMap::new();
    let mut client_id_counter = 1;

    loop {
        tokio::select! {
            Ok((socket, _)) = listener.accept() => {
                let client_id: ClientId = client_id_counter;
                client_id_counter += 1;

                let (client_tx, client_rx) = mpsc::unbounded_channel::<Vec<u8>>();
                // let event_tx_clone = event_tx.clone();
                //let client_clone = client.clone();

                clients.insert(client_id, client_tx);
                //event_tx_clone.send(ClientEvent::NewClient { client_id }).unwrap();

                tokio::spawn(handle_client(client_id, socket, client_rx, event_tx.clone(), network_tx.clone()));
            }

            Some(msg) = network_rx.recv() => {
                match msg {
                    NetworkMessage::SendPacket { client_id, packet } => {
                        if let Some(client_tx) = clients.get(&client_id) {
                            //println!("sending packet to client {}: {:?}", client_id, packet);
                            match packet.encode().await {
                                Ok(bytes) => {
                                    let _ = client_tx.send(bytes);
                                }
                                Err(e) => {
                                    eprintln!("Failed to encode packet for client {}: {}", client_id, e);
                                }
                            }
                        } else {
                            eprintln!("Attempted to send packet to nonexistent client {}", client_id);
                        }
                    }
                    NetworkMessage::DisconnectClient { client_id } => {
                        clients.remove(&client_id);
                    }
                }
            }
        }
    }
}
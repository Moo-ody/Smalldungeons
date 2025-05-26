use crate::net::client::{handle_client, Client};
use std::collections::HashMap;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::net::client_event::ClientEvent;
use crate::net::connection_state::ConnectionState;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::packet::ClientBoundPacket;

pub async fn run_network_thread(
    mut network_rx: UnboundedReceiver<NetworkMessage>,
    network_tx: UnboundedSender<NetworkMessage>,
    event_tx: UnboundedSender<ClientEvent>,
) {
    let listener = TcpListener::bind("127.0.0.1:4972").await.unwrap();
    println!("Network thread listening on 127.0.0.1:4972");

    let mut clients: HashMap<u32, (Client, UnboundedSender<Vec<u8>>)> = HashMap::new();
    let mut client_id_counter = 0;

    loop {
        tokio::select! {
            Ok((socket, _)) = listener.accept() => {
                let client_id = client_id_counter;
                client_id_counter += 1;

                let client = Client {
                    client_id,
                    connection_state: ConnectionState::Handshaking
                };

                let (client_tx, client_rx) = mpsc::unbounded_channel::<Vec<u8>>();
                let event_tx_clone = event_tx.clone();
                let client_clone = client.clone();

                clients.insert(client_id, (client.clone(), client_tx.clone()));
                //event_tx_clone.send(ClientEvent::NewClient { client_id }).unwrap();

                tokio::spawn(handle_client(client.client_id, socket, client_rx, event_tx_clone, network_tx.clone()));
            }

            Some(msg) = network_rx.recv() => {
                match msg {
                    NetworkMessage::SendPacket { client_id, packet } => {
                        if let Some((_, client_tx)) = clients.get(&client_id) {
                            println!("sending packet to client {}: {:?}", client_id, packet);
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
                    NetworkMessage::UpdateConnectionState { client_id, new_state } => {
                        if let Some((client, _)) = clients.get_mut(&client_id) {
                            client.connection_state = new_state;
                        }
                    }
                    NetworkMessage::GetConnectionState { client_id, response } => {
                        if let Some((client, _)) = clients.get(&client_id) {
                            let _ = response.send(client.connection_state.clone());
                        }
                    }
                }
            }
        }
    }
}
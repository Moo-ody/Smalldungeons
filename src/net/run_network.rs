use crate::net::client::handle_client;
use std::collections::HashMap;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::net::internal_packets::{ClientHandlerMessage, MainThreadMessage, NetworkThreadMessage};
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::player::player::ClientId;

pub async fn run_network_thread(
    mut network_rx: UnboundedReceiver<NetworkThreadMessage>,
    network_tx: UnboundedSender<NetworkThreadMessage>,
    main_tx: UnboundedSender<MainThreadMessage>,
) {
    
    let listener = TcpListener::bind("127.0.0.1:4972").await.unwrap();
    println!("Network thread listening on 127.0.0.1:4972");

    let mut clients: HashMap<ClientId, UnboundedSender<ClientHandlerMessage>> = HashMap::new();
    let mut client_id_counter = 1;

    loop {
        tokio::select! {
            Ok((socket, _)) = listener.accept() => {
                let client_id: ClientId = client_id_counter;
                client_id_counter += 1;

                let (client_tx, client_rx) = mpsc::unbounded_channel::<ClientHandlerMessage>();

                clients.insert(client_id, client_tx);
                tokio::spawn(handle_client(client_id, socket, client_rx, main_tx.clone(), network_tx.clone()));
            }

            Some(msg) = network_rx.recv() => {
                match msg {
                    NetworkThreadMessage::SendPacket { client_id, packet } => {
                        if let Some(client_tx) = clients.get(&client_id) {
                            //println!("sending packet to client {}: {:?}", client_id, packet);
                            match packet.encode().await {
                                Ok(bytes) => {
                                    let _ = client_tx.send(ClientHandlerMessage::Send(bytes));
                                }
                                Err(e) => {
                                    eprintln!("Failed to encode packet for client {}: {}", client_id, e);
                                }
                            }
                        } else {
                            eprintln!("Attempted to send packet to nonexistent client {}", client_id);
                        }
                    }

                    NetworkThreadMessage::DisconnectClient { client_id } => {
                        if let Some(client_tx) = clients.get(&client_id) {
                            let _ = client_tx.send(ClientHandlerMessage::CloseHandler);
                        } else {
                            eprintln!("Attempted to disconnect nonexistent client {}", client_id);
                        }
                    }

                    NetworkThreadMessage::ConnectionClosed { client_id } => {
                        let _ = main_tx.send(MainThreadMessage::ClientDisconnected { client_id });
                        clients.remove(&client_id);
                    }
                }
            }
        }
    }
}
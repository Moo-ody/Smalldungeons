use crate::net::client_event::ClientEvent;
use crate::net::connection_state::ConnectionState;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::packets::packet_registry::parse_packet;
use crate::server::player::ClientId;
use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct Client {
    pub client_id: ClientId,
    pub connection_state: ConnectionState,
}

pub async fn handle_client(
    client_id: ClientId,
    socket: TcpStream,
    mut rx: UnboundedReceiver<Vec<u8>>,
    event_tx: UnboundedSender<ClientEvent>,
    network_tx: UnboundedSender<NetworkMessage>,
) {
    let (mut reader, mut writer) = tokio::io::split(socket);

    let write_task = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if let Err(e) = writer.write_all(&data).await {
                eprintln!("write error: {e}");
                break
            }
        }
    });

    let mut buf = [0u8; 1024];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let mut bytes = BytesMut::from(&buf[..n]);

                let (connection_state, event_tx_clone) = {
                    let (sender, receiver) = tokio::sync::oneshot::channel();
                    network_tx.send(NetworkMessage::GetConnectionState {
                        client_id,
                        response: sender,
                    }).unwrap();

                    (receiver.await.unwrap(), event_tx.clone())
                };

                let client_stub = Client {
                    client_id,
                    connection_state,
                };

                match parse_packet(&mut bytes, &client_stub).await {
                    Ok(packet) => {
                        if let Err(e) = packet.process(PacketContext {
                            client_id,
                            network_tx: network_tx.clone(),
                            event_tx: event_tx_clone.clone(),
                        }).await
                        {
                            eprintln!("Failed to process packet for {client_id}: {e}");
                            continue;
                        }

                        if client_stub.connection_state == ConnectionState::Play {
                            event_tx_clone.send(ClientEvent::PacketReceived { client_id, packet })
                                .unwrap_or_else(|e| eprintln!("Failed to send packet to main thread from {client_id}: {e}"));
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse packet from client {client_id}: {e}");
                        continue;
                    }
                }
            }
            Err(e) => {
                eprintln!("Client {client_id} read error: {e}");
                break;
            }
        }
    }

    println!("handle client for {client_id} closed.");
    event_tx.send(ClientEvent::ClientDisconnected { client_id }).unwrap();
    write_task.abort();
}
use crate::net::connection_state::ConnectionState;
use crate::net::internal_packets::{ClientHandlerMessage, MainThreadMessage, NetworkThreadMessage};
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::packets::packet_registry::parse_packet;
use crate::net::var_int::peek_var_int;
use crate::server::player::ClientId;
use bytes::{Buf, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub struct Client {
    client_id: ClientId,
    pub connection_state: ConnectionState,
}

impl Client {
    pub const fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            connection_state: ConnectionState::Handshaking,
        }
    }

    pub const fn client_id(&self) -> ClientId {
        self.client_id
    }
}

pub async fn handle_client(
    client_id: ClientId,
    mut socket: TcpStream,
    mut rx: UnboundedReceiver<ClientHandlerMessage>,
    main_tx: UnboundedSender<MainThreadMessage>,
    network_tx: UnboundedSender<NetworkThreadMessage>,
) {
    let mut client = Client::new(client_id);
    let mut bytes = BytesMut::new();

    loop {
        tokio::select! {
            result = socket.read_buf(&mut bytes) => {
                match result {
                    Ok(0) => { break },
                    Ok(_) => {
                        while let Some(mut packet) = read_whole_packet(&mut bytes).await {
                            match parse_packet(&mut packet, &mut client).await {
                                Ok(parsed) => {
                                    if let Err(e) = parsed.process(PacketContext {
                                        client: &mut client,
                                        network_tx: &network_tx,
                                        main_tx: &main_tx,
                                    }).await
                                    {
                                        eprintln!("Failed to process packet for {client_id}: {e}");
                                        continue;
                                    }

                                    if client.connection_state == ConnectionState::Play {
                                        main_tx.send(MainThreadMessage::PacketReceived { client_id, packet: parsed })
                                            .unwrap_or_else(|e| eprintln!("Failed to send packet to main thread from {client_id}: {e}"));
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to parse packet from client {client_id}: {e}");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Client {client_id} read error: {e}");
                        break;
                    }
                }
            }

            Some(message) = rx.recv() => {
                match message {
                    ClientHandlerMessage::Send(data) => {
                        if let Err(e) = socket.write_all(&data).await {
                            eprintln!("write error: {e}");
                            break
                        }
                    }

                    ClientHandlerMessage::CloseHandler => {
                        break
                    }
                }
            }
        }
    }

    network_tx.send(NetworkThreadMessage::ConnectionClosed { client_id }).unwrap();
    println!("handle client for {client_id} closed.");
}

pub async fn read_whole_packet(buf: &mut BytesMut) -> Option<BytesMut> {
    if buf.is_empty() {
        return None;
    }
    let (packet_len, varint_len) = peek_var_int(buf)?;

    let packet_len = packet_len as usize;
    if buf.len() < packet_len + varint_len {
        return None;
    }

    buf.advance(varint_len);
    Some(buf.split_to(packet_len))
}
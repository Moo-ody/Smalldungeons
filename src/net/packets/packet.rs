use crate::net::network_message::NetworkMessage;
use crate::net::packets::packet_context::PacketContext;
use crate::server::player::ClientId;
use anyhow::Result;
use bytes::BytesMut;
use tokio::io::AsyncWrite;
use tokio::sync::mpsc::UnboundedSender;
use crate::net::var_int::write_var_int;

#[macro_export]
macro_rules! register_clientbound_packets {
    { $($packet_ty:ident),* $(,)? } => {

        #[derive(Debug, Clone)]
        pub enum ClientBoundPacket {
            $(
                $packet_ty($packet_ty),
            )*
        }
        
        $(
            impl From<$packet_ty> for ClientBoundPacket {
                fn from(pkt: $packet_ty) -> Self {
                    ClientBoundPacket::$packet_ty(pkt)
                }
            }
        
            impl crate::net::packets::packet::SendPacket<$packet_ty> for $packet_ty {
                fn send_packet(self, client_id: crate::server::player::ClientId, network_tx: &tokio::sync::mpsc::UnboundedSender<crate::net::network_message::NetworkMessage>) -> anyhow::Result<()> {
                    // println!("Sending packet {:?} to client {}", self, client_id);
                    ClientBoundPacket::$packet_ty(self).send_packet(client_id, network_tx)
                    
                }
            }
        )*

        #[async_trait::async_trait]
        impl crate::net::packets::packet::ClientBoundPacketImpl for ClientBoundPacket {
            async fn write_to<W: tokio::io::AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> tokio::io::Result<()> {
                match self {
                    $(
                        ClientBoundPacket::$packet_ty(pkt) => pkt.write_to(writer).await,
                    )*
                }
            }

            async fn encode(&self) -> anyhow::Result<Vec<u8>> {
                match self {
                    $(
                        ClientBoundPacket::$packet_ty(pkt) => pkt.encode().await,
                    )*
                }
            }
        }

        impl ClientBoundPacket {
            pub fn send_packet(self, client_id: crate::server::player::ClientId, network_tx: &tokio::sync::mpsc::UnboundedSender<crate::net::network_message::NetworkMessage>) -> anyhow::Result<()> {
                network_tx.send(crate::net::network_message::NetworkMessage::SendPacket {
                    client_id,
                    packet: self
                })?;
                Ok(())
            }
        }
    }
}

pub trait SendPacket<T> where T: Sized {
    fn send_packet(self, client_id: ClientId, network_tx: &UnboundedSender<NetworkMessage>) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait ClientBoundPacketImpl: Send + Sync {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> tokio::io::Result<()>;

    async fn encode(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.write_to(&mut buf).await?;
        Ok(buf)
    }
}

#[macro_export]
macro_rules! register_serverbound_packets {
    (
        $(
            $state:path {
                $(
                    $id:expr => $packet_ty:ident,
                )*
            }
        ),* $(,)?
    ) => {
        use crate::net::packets::packet_context::PacketContext;
        use crate::net::client::Client;
        use crate::net::packets::packet::ServerBoundPacket;
        use crate::net::var_int::read_var_int;
        use anyhow::{bail};
        use bytes::BytesMut;

        #[derive(Debug)]
        pub enum ServerBoundPackets {
            $(
                $(
                    $packet_ty($packet_ty),
                )*
            )*
        }

        #[async_trait::async_trait]
        impl ServerBoundPacket for ServerBoundPackets {
            async fn read_from(_buf: &mut BytesMut) -> anyhow::Result<Self> {
                unimplemented!("Use parse_packet instead");
            }

            async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
                match self {
                    $(
                        $(
                            ServerBoundPackets::$packet_ty(pkt) => pkt.process(context).await,
                        )*
                    )*
                }
            }

            fn main_process(&self, world: &mut crate::server::world::World, player: &mut crate::server::player::Player) -> anyhow::Result<()> {
                match self {
                    $(
                        $(
                            ServerBoundPackets::$packet_ty(pkt) => pkt.main_process(world, player),
                        )*
                    )*
                }
            }
        }

        pub async fn parse_packet(buf: &mut BytesMut, client: &Client) -> anyhow::Result<ServerBoundPackets> {
            let _packet_len = read_var_int(buf).unwrap_or(0);
            let packet_id = read_var_int(buf).ok_or_else(|| anyhow::anyhow!("Failed to read packet id"))?;

            match client.connection_state {
                $(
                    $state => match packet_id {
                        $(
                            $id => {
                                // println!("Received from {} packet id {} for state {:?}", client.client_id, packet_id, stringify!($state));
                                let pkt = $packet_ty::read_from(buf).await?;
                                let packet = ServerBoundPackets::$packet_ty(pkt);
                                Ok(packet)
                            }
                        )*
                        _ => bail!("Unknown packet id {} for state {:?}", packet_id, stringify!($state)),
                    },
                )*
            }
        }
    };
}

#[macro_export]
macro_rules! print_bytes_hex {
    ($ident:tt, $buf:expr) => {
        println!("Raw bytes for {} [{}]: {}", $ident, $buf.len(), $buf.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(" "));};
}

#[async_trait::async_trait]
pub trait ServerBoundPacket: Send + Sync {
    async fn read_from(buf: &mut BytesMut) -> Result<Self> where Self: Sized;

    async fn process(&self, _: PacketContext) -> Result<()> {
        Ok(())
    }

    fn main_process(&self, _: &mut crate::server::world::World, _: &mut crate::server::player::Player) -> Result<()> {
        Ok(())
    }
}

#[macro_export]
macro_rules! build_packet {
    ($packet_id:expr $(, $value:expr )* $(,)?) => {{
        let mut buf = Vec::new();
        let mut payload = Vec::new();

        $crate::net::var_int::write_var_int(&mut payload, $packet_id);

        $(
            $crate::net::packets::packet_write::PacketWrite::write(&$value, &mut payload);
        )*

        $crate::net::var_int::write_var_int(&mut buf, payload.len() as i32);
        buf.extend_from_slice(&payload);

        buf
    }};
}

#[macro_export]
macro_rules! partial_packet {
    ($buf:expr => $($value:expr),* $(,)?) => {{
        $(
            $crate::net::packets::packet_write::PacketWrite::write(&$value, &mut $buf);
        )*
    }}
}

/// appends the length of the payload and then the payload to the returned buffer.
pub fn finish_packet(payload: Vec<u8>) -> Vec<u8> {
    let mut buf = Vec::new();
    write_var_int(&mut buf, payload.len() as i32);
    buf.extend_from_slice(&payload);
    buf
}
use crate::net::network_message::NetworkMessage;
use crate::net::packets::packet_context::PacketContext;
use crate::net::varint::write_varint;
use crate::server::entity::metadata::Metadata;
use anyhow::Result;
use bytes::BytesMut;
use tokio::io::AsyncWrite;
use tokio::sync::mpsc::UnboundedSender;

#[macro_export]
macro_rules! register_clientbound_packets {
    { $($packet_ty:ident),* $(,)? } => {

        #[derive(Debug)]
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
                fn send_packet(self, client_id: u32, network_tx: &tokio::sync::mpsc::UnboundedSender<crate::net::network_message::NetworkMessage>) -> anyhow::Result<()> {
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
            pub fn send_packet(self, client_id: u32, network_tx: &tokio::sync::mpsc::UnboundedSender<crate::net::network_message::NetworkMessage>) -> anyhow::Result<()> {
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
    fn send_packet(self, client_id: u32, network_tx: &UnboundedSender<NetworkMessage>) -> anyhow::Result<()>;
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
        use crate::net::varint::read_varint;
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

            fn main_process(&self, world: &mut crate::server::old_world::World, client_id: u32) -> anyhow::Result<()> {
                match self {
                    $(
                        $(
                            ServerBoundPackets::$packet_ty(pkt) => pkt.main_process(world, client_id),
                        )*
                    )*
                }
            }
        }

        pub async fn parse_packet(buf: &mut BytesMut, client: &Client) -> anyhow::Result<ServerBoundPackets> {
            let hex_string: String = buf.iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<String>>()
                .join(" ");

            println!("Raw bytes [{}]: {}", buf.len(), hex_string);

            let _packet_len = read_varint(buf).unwrap_or(0);
            let packet_id = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read packet id"))?;

            match client.connection_state {
                $(
                    $state => match packet_id {
                        $(
                            $id => {
                                println!("Received packet id {} for state {:?}", packet_id, stringify!($state));
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

#[async_trait::async_trait]
pub trait ServerBoundPacket: Send + Sync {
    async fn read_from(buf: &mut BytesMut) -> Result<Self> where Self: Sized;

    async fn process(&self, context: PacketContext) -> Result<()>;

    fn main_process(&self, world: &mut crate::server::old_world::World, client_id: u32) -> Result<()>;
}

#[macro_export]
macro_rules! build_packet {
    ($packet_id:expr $(, $value:expr )* $(,)?) => {{
        let mut buf = Vec::new();
        let mut payload = Vec::new();

        $crate::net::varint::write_varint(&mut payload, $packet_id);

        $(
            $crate::net::packets::packet::PacketWrite::write(&$value, &mut payload);
        )*

        $crate::net::varint::write_varint(&mut buf, payload.len() as i32);
        buf.extend_from_slice(&payload);

        buf
    }};
}

pub trait PacketWrite {
    fn write(&self, buf: &mut Vec<u8>);
}

impl PacketWrite for Metadata {
    fn write(&self, buf: &mut Vec<u8>) {
        for data in self {
            data.write_to_buffer(buf)
        }
        buf.push(127);
    }
}

impl PacketWrite for bool {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.push(*self as u8)
    }
}

impl PacketWrite for u8 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.push(*self);
    }
}

impl PacketWrite for i8 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.push(*self as u8);
    }
}

impl PacketWrite for u16 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for i16 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for u32 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for i32 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for i64 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for f32 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for f64 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for &[u8] {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self);
    }
}

impl<const N: usize> PacketWrite for &[u8; N] {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self[..]);
    }
}

impl PacketWrite for &str {
    fn write(&self, buf: &mut Vec<u8>) {
        write_varint(buf, self.len() as i32);
        buf.extend_from_slice(self.as_bytes());
    }
}

impl PacketWrite for String {
    fn write(&self, buf: &mut Vec<u8>) {
        write_varint(buf, self.len() as i32);
        buf.extend_from_slice(self.as_bytes());
    }
}
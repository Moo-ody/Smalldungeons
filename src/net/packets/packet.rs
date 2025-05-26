use std::any::Any;
use bytes::BytesMut;
use anyhow::Result;
use tokio::io::{AsyncWrite, WriteHalf};
use crate::net::client::Client;
use crate::net::packets::packet_context::PacketContext;
use crate::net::varint::write_varint;

#[macro_export]
macro_rules! register_clientbound_packets {
    { $($packet_ty:ident),* $(,)? } => {
        use crate::net::packets::packet::ClientBoundPacket;
        use tokio::io::AsyncWrite;
        use anyhow::Result;

        #[derive(Debug)]
        pub enum ClientBoundPackets {
            $(
                $packet_ty($packet_ty),
            )*
        }

        #[async_trait::async_trait]
        impl ClientBoundPacket for ClientBoundPackets {
            async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> tokio::io::Result<()> {
                match self {
                    $(
                        ClientBoundPackets::$packet_ty(pkt) => pkt.write_to(writer).await,
                    )*
                }
            }

            async fn encode(&self) -> Result<Vec<u8>> {
                match self {
                    $(
                        ClientBoundPackets::$packet_ty(pkt) => pkt.encode().await,
                    )*
                }
            }
        }
    }
}

#[async_trait::async_trait]
pub trait ClientBoundPacket: Send + Sync {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> tokio::io::Result<()>;
    
    async fn encode(&self) -> Result<Vec<u8>> {
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
        use anyhow::{Result, bail};
        use bytes::BytesMut;
        use std::any::Any;

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
            async fn read_from(_buf: &mut BytesMut) -> Result<Self> where Self: Sized {
                unimplemented!("Use parse_packet instead");
            }

            fn as_any(&self) -> &dyn Any {
                match self {
                    $(
                        $(
                            ServerBoundPackets::$packet_ty(pkt) => pkt.as_any(),
                        )*
                    )*
                }
            }

            async fn process(&self, context: PacketContext) -> Result<()> {
                match self {
                    $(
                        $(
                            ServerBoundPackets::$packet_ty(pkt) => pkt.process(context).await,
                        )*
                    )*
                }
            }
        }

        pub async fn parse_packet(buf: &mut BytesMut, client: &Client) -> Result<ServerBoundPackets> {
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
                                let pkt = $packet_ty::read_from(buf).await?;
                                Ok(ServerBoundPackets::$packet_ty(pkt))
                            }
                        )*
                        _ => bail!("Unknown packet id {} for state {:?}", packet_id, stringify!($state)),
                    },
                )*
                _ => bail!("Unknown connection state: {:?}", client.connection_state),
            }
        }
    };
}

#[async_trait::async_trait]
pub trait ServerBoundPacket: Send + Sync {
    async fn read_from(buf: &mut BytesMut) -> Result<Self> where Self: Sized;

    fn as_any(&self) -> &dyn Any;

    async fn process(&self, context: PacketContext) -> Result<()>;
}

#[macro_export]
macro_rules! build_packet {
    ($packet_id:expr $(, $value:expr )* $(,)?) => {{
        let mut buf = Vec::new();
        let mut payload = Vec::new();

        // Write packet ID
        $crate::net::varint::write_varint(&mut payload, $packet_id);

        $(
            $crate::net::packets::packet::PacketWrite::write(&$value, &mut payload);
        )*

        // Prepend length
        $crate::net::varint::write_varint(&mut buf, payload.len() as i32);
        buf.extend_from_slice(&payload);

        buf
    }};
}

// Trait to write values to the buffer
pub trait PacketWrite {
    fn write(&self, buf: &mut Vec<u8>);
}

impl PacketWrite for bool {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.push(*self as u8)
    }
}

// Basic numeric types
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

// Byte slices
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

// Strings
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
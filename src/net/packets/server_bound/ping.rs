use std::any::Any;
use anyhow::{Result, bail};
use bytes::{Buf, BytesMut};
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::packet_registry::ClientBoundPackets;
use crate::net::packets::client_bound::pong::Pong;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::varint::read_varint;

#[derive(Debug)]
pub struct Ping {
    pub client_time: i64
}

#[async_trait::async_trait]
impl ServerBoundPacket for Ping {
    async fn read_from(buf: &mut BytesMut) -> Result<Self> {
        // let packet_id = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read packet id"))?;
        // if packet_id != 0x01 {
        //     bail!("Expected Ping packet (0x01), got {}", packet_id);
        // }

        if buf.len() < 8 {
            bail!("Buffer too small for ping payload");
        }

        let client_time = buf.get_i64();
        Ok(Ping { client_time })

    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn process(&self, context: PacketContext) -> Result<()> {
        println!("Received ping: {}", self.client_time);
        
        context.network_tx.send(NetworkMessage::SendPacket {
            client_id: context.client_id,
            packet: ClientBoundPackets::Pong(Pong {
                client_time: self.client_time,
            }),
        })?;
        Ok(())
    }
}
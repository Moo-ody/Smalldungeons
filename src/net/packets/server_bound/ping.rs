use crate::net::packets::packet_registry::ClientBoundPackets;
use crate::net::packets::client_bound::pong::Pong;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use anyhow::{bail, Result};
use bytes::{Buf, BytesMut};

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

    async fn process(&self, context: PacketContext) -> Result<()> {
        println!("Received ping: {}", self.client_time);

        ClientBoundPackets::Pong(Pong {
            client_time: self.client_time,
        }).send_packet(context.client_id, &context.network_tx)?;

        Ok(())
    }
}
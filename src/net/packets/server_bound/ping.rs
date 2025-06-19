use crate::net::packets::client_bound::pong::Pong;
use crate::net::packets::packet::{SendPacket, ServerBoundPacket};
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
        if buf.len() < 8 {
            bail!("Buffer too small for ping payload");
        }

        let client_time = buf.get_i64();
        Ok(Self { client_time })
    }

    async fn process<'a>(&self, context: PacketContext<'a>) -> Result<()> {
        println!("Received ping: {}", self.client_time);

        Pong {
            client_time: self.client_time,
        }.send_packet(context.client.client_id(), context.network_tx)?;

        Ok(())
    }
}
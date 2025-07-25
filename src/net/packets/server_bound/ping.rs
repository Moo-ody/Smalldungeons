use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::packets::old_packet::ServerBoundPacket;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::packets::packet_context::PacketContext;
use crate::net::packets::protocol::clientbound::Pong;
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

        // todo, a way to write individual packets for cases like these (shouldn't be used commonly tho)
        let mut buffer = PacketBuffer { buf: Vec::new() };
        buffer.write_packet(&Pong {
            client_time: self.client_time,
        });
        context.network_tx.send(NetworkThreadMessage::SendPackets {
            client_id: context.client.client_id(),
            buffer: buffer.buf,
        })?;
        Ok(())
    }
}
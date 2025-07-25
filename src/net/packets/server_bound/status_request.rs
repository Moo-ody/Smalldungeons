use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::packets::old_packet::ServerBoundPacket;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::packets::packet_context::PacketContext;
use crate::net::packets::protocol::clientbound::ServerInfo;
use crate::STATUS_RESPONSE_JSON;
use anyhow::Result;
use bytes::BytesMut;

#[derive(Debug)]
pub struct StatusRequest;

#[async_trait::async_trait]
impl ServerBoundPacket for StatusRequest {
    async fn read_from(buf: &mut BytesMut) -> Result<Self> {
        Ok(Self)
    }

    async fn process<'a>(&self, context: PacketContext<'a>) -> Result<()> {
        // todo improve
        let mut buffer = PacketBuffer { buf: Vec::new() };
        buffer.write_packet(&ServerInfo {
            status: STATUS_RESPONSE_JSON.parse()?,
        });
        context.network_tx.send(NetworkThreadMessage::SendPackets {
            client_id: context.client.client_id(),
            buffer: buffer.buf,
        })?;
        Ok(())
    }
}
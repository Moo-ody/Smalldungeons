use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use bytes::BytesMut;

#[derive(Debug)]
pub struct KeepAlive;

#[async_trait::async_trait]
impl ServerBoundPacket for KeepAlive {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        todo!()
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        todo!()
    }
}
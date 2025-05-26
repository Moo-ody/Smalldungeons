use std::any::Any;
use bytes::BytesMut;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;

#[derive(Debug)]
pub struct KeepAlive;

#[async_trait::async_trait]
impl ServerBoundPacket for KeepAlive {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        todo!()
    }
}
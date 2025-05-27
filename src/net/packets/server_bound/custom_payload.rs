use bytes::BytesMut;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::server::world::World;

pub struct CustomPayload;

#[async_trait::async_trait]
impl ServerBoundPacket for CustomPayload {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        todo!()
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        todo!()
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        todo!()
    }
}
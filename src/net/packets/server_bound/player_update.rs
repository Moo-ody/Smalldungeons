use bytes::{Buf, BytesMut};
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::server::world::World;

#[derive(Debug)]
pub struct PlayerUpdate {
    on_ground: bool,
}

#[async_trait::async_trait]
impl ServerBoundPacket for PlayerUpdate {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(PlayerUpdate {
            on_ground: buf.get_u8() != 0
        })
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        Ok(())
    }
}
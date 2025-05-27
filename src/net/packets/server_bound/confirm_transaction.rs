use bytes::{Buf, BytesMut};
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::server::world::World;

#[derive(Debug)]
pub struct ConfirmTransaction {
    pub window_id: i8,
    pub action_number: i16,
    pub accepted: bool,
}

#[async_trait::async_trait]
impl ServerBoundPacket for ConfirmTransaction {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(ConfirmTransaction {
            window_id: buf.get_i8(),
            action_number: buf.get_i16(),
            accepted: buf.get_i8() != 0,
        })
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        Ok(())
    }
}
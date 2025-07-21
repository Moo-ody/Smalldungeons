use crate::net::packets::packet::ServerBoundPacket;
use crate::server::player::player::Player;
use crate::server::world::World;
use bytes::{Buf, BytesMut};

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

    fn main_process(&self, world: &mut World, player: &mut Player) -> anyhow::Result<()> {
        Ok(())
    }
}
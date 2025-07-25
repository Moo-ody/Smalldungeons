use crate::net::packets::old_packet::ServerBoundPacket;
use crate::server::player::player::Player;
use crate::server::world::World;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct PlayerLook {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub rotating: bool,
}

#[async_trait::async_trait]
impl ServerBoundPacket for PlayerLook {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(Self {
            yaw: buf.get_f32(),
            pitch: buf.get_f32(),
            on_ground: buf.get_u8() != 0,
            rotating: true,
        })
    }

    fn main_process(&self, _: &mut World, player: &mut Player) -> anyhow::Result<()> {
        player.yaw = self.yaw;
        player.pitch = self.pitch;
        Ok(())
    }
}
use crate::net::packets::old_packet::ServerBoundPacket;
use crate::server::player::player::Player;
use crate::server::world::World;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct PlayerPosLook {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub moving: bool,
    pub rotating: bool,
}

#[async_trait::async_trait]
impl ServerBoundPacket for PlayerPosLook {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(Self {
            x: buf.get_f64(),
            y: buf.get_f64(),
            z: buf.get_f64(),
            yaw: buf.get_f32(),
            pitch: buf.get_f32(),
            on_ground: buf.get_u8() != 0,
            moving: true,
            rotating: true,       
        })
    }

    fn main_process(&self, _: &mut World, player: &mut Player) -> anyhow::Result<()> {
        player.set_position(self.x, self.y, self.z);
        player.yaw = self.yaw;
        player.pitch = self.pitch;
        Ok(())
    }
}
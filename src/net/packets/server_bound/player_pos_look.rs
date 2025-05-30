use crate::net::packets::packet::ServerBoundPacket;
use crate::server::player::Player;
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

    fn main_process(&self, world: &mut World, player: &mut Player) -> anyhow::Result<()> {
        let entity = player.get_entity(world)?;
        entity.update_position(self.x, self.y, self.z);
        entity.yaw = self.yaw;
        entity.pitch = self.pitch;
        Ok(())
    }
}
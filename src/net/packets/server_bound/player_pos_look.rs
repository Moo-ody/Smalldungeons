use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::server::entity::entity_enum::EntityTrait;
use crate::server::old_world::World;
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
        Ok(PlayerPosLook {
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

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        let entity = world.get_player_from_client_id(client_id)?.get_entity();
        entity.update_position(self.x, self.y, self.z);
        entity.yaw = self.yaw;
        entity.pitch = self.pitch;
        Ok(())
    }
}
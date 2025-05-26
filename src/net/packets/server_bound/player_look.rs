use bytes::{Buf, BytesMut};
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::server::entity::entity_enum::EntityTrait;
use crate::server::world::World;

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
        Ok(PlayerLook {
            yaw: buf.get_f32(),
            pitch: buf.get_f32(),
            on_ground: buf.get_u8() != 0,
            rotating: true,
        })
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        if let Some(player) = world.get_player_from_client_id(client_id) {
            let entity = player.get_entity();
            entity.yaw = self.yaw;
            entity.pitch = self.pitch;
        }
        Ok(())
    }
}
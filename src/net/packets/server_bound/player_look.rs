use crate::net::packets::packet::ServerBoundPacket;
use crate::server::player::Player;
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
        Ok(PlayerLook {
            yaw: buf.get_f32(),
            pitch: buf.get_f32(),
            on_ground: buf.get_u8() != 0,
            rotating: true,
        })
    }

    fn main_process(&self, world: &mut World, player: &mut Player) -> anyhow::Result<()> {
        let entity = world.entities.get_mut(&player.entity_id).ok_or_else(|| anyhow::anyhow!("Player {player:?}'s entity not found"))?;
        entity.yaw = self.yaw;
        entity.pitch = self.pitch;
        Ok(())
    }
}
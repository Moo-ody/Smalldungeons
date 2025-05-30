use crate::net::packets::packet::ServerBoundPacket;
use crate::server::player::Player;
use crate::server::world::World;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct PlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
    pub moving: bool,
}

#[async_trait::async_trait]
impl ServerBoundPacket for PlayerPosition {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(PlayerPosition {
            x: buf.get_f64(),
            y: buf.get_f64(),
            z: buf.get_f64(),
            on_ground: buf.get_u8() != 0,
            moving: true,
        })
    }

    fn main_process(&self, world: &mut World, player: &mut Player) -> anyhow::Result<()> {
        let entity = world.entities.get_mut(&player.entity_id).ok_or_else(|| anyhow::anyhow!("Player {player:?}'s entity not found"))?;
        entity.update_position(self.x, self.y, self.z);
        Ok(())
    }
}
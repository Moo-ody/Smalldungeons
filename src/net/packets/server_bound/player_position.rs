use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::server::entity::entity_enum::EntityTrait;
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

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        world.get_player_from_client_id(client_id)?.get_entity().update_position(self.x, self.y, self.z);
        Ok(())
    }
}
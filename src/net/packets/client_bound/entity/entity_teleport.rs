use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use crate::server::entity::entity::Entity;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug, Clone, Copy)]
pub struct EntityTeleport {
    entity_id: i32,
    pos_x: i32,
    pos_y: i32,
    pos_z: i32,
    yaw: i8,
    pitch: i8,
    on_ground: bool,
}

impl EntityTeleport {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            entity_id: entity.entity_id,
            pos_x: (entity.pos.x * 32.0) as i32,
            pos_y: (entity.pos.y * 32.0) as i32,
            pos_z: (entity.pos.z * 32.0) as i32,
            yaw: (entity.yaw * 256.0 / 360.0) as i8,
            pitch: (entity.pitch * 256.0 / 360.0) as i8,
            on_ground: true, // todo replace with entity.grounded or smth.
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityTeleport {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x18,
            VarInt(self.entity_id),
            self.pos_x,
            self.pos_y,
            self.pos_z,
            self.yaw,
            self.pitch,
            self.on_ground
        );
        writer.write_all(&buf).await
    }
}
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::var_int::VarInt;
use crate::server::entity::entity::Entity;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug, Clone, Copy)]
pub struct EntityLookMove {
    entity_id: i32,
    pos_x: i8,
    pos_y: i8,
    pos_z: i8,
    yaw: i8,
    pitch: i8,
    on_ground: bool,
}

impl EntityLookMove {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            entity_id: entity.id,
            pos_x: ((entity.last_position.x - entity.position.x) * 32.0) as i8,
            pos_y: ((entity.last_position.y - entity.position.y) * 32.0) as i8,
            pos_z: ((entity.last_position.z - entity.position.z) * 32.0) as i8,
            yaw: ((entity.last_yaw - entity.yaw) * 256.0 / 360.0) as i8,
            pitch: ((entity.last_pitch - entity.pitch) * 256.0 / 360.0) as i8,
            on_ground: entity.on_ground,
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityLookMove {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x17,
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
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use crate::server::entity::entity::Entity;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug)]
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
    pub fn from_entity(entity: &mut Entity) -> Self {
        let to_send_pos = entity.last_sent_pos - entity.pos;  // TEMPORARY, this will and should be different for every player.
        let to_send_yaw = entity.last_sent_yaw - entity.yaw;
        let to_send_pitch = entity.last_sent_pitch - entity.pitch;
        entity.last_sent_pos = entity.pos;
        entity.last_sent_yaw = entity.yaw;
        entity.last_sent_pitch = entity.pitch;
        Self {
            entity_id: entity.entity_id,
            pos_x: (to_send_pos.x * 32.0) as i8,
            pos_y: (to_send_pos.y * 32.0) as i8,
            pos_z: (to_send_pos.z * 32.0) as i8,
            yaw: (to_send_yaw * 256.0 / 360.0) as i8,
            pitch: (to_send_pitch * 256.0 / 360.0) as i8,
            on_ground: true,
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
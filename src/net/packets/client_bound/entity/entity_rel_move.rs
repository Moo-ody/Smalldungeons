use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::var_int::VarInt;
use crate::server::entity::entity::Entity;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug, Clone, Copy)]
pub struct EntityRelMove {
    entity_id: i32,
    pos_x: i8,
    pos_y: i8,
    pos_z: i8,
    on_ground: bool,
}

impl EntityRelMove {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            entity_id: entity.entity_id,
            pos_x: ((entity.last_sent_pos.x - entity.pos.x) * 32.0) as i8,
            pos_y: ((entity.last_sent_pos.y - entity.pos.y) * 32.0) as i8,
            pos_z: ((entity.last_sent_pos.z - entity.pos.z) * 32.0) as i8,
            on_ground: entity.on_ground,
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityRelMove {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x15,
            VarInt(self.entity_id),
            self.pos_x,
            self.pos_y,
            self.pos_z,
            self.on_ground
        );
        writer.write_all(&buf).await
    }
}
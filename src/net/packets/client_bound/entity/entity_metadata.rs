use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::var_int::VarInt;
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_metadata::EntityMetadata;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug, Clone)]
pub struct PacketEntityMetadata {
    pub entity_id: i32,
    pub metadata: EntityMetadata,
}

impl PacketEntityMetadata {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            entity_id: entity.id,
            metadata: entity.metadata.clone(),
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for PacketEntityMetadata {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x1C,
            VarInt(self.entity_id),
            self.metadata
        );
        writer.write_all(&buf).await
    }
}
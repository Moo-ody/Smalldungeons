use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use crate::server::entity::entity::Entity;
use crate::server::entity::metadata::Metadata;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

pub struct EntityMetadata {
    entity_id: i32,
    metadata: Vec<Metadata>,
}

impl EntityMetadata {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            entity_id: entity.entity_id,
            metadata: entity.metadata.clone(),
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityMetadata {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x1C,
            VarInt(self.entity_id),
            self.metadata
        );
        writer.write_all(&buf).await
    }
}
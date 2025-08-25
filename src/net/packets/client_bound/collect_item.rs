use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::var_int::VarInt;
use crate::server::entity::entity::EntityId;
use async_trait::async_trait;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Copy, Clone)]
pub struct PacketCollectItem {
    pub item_entity_id: EntityId,
    pub entity_id: EntityId,
}

#[async_trait]
impl ClientBoundPacketImpl for PacketCollectItem {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x0d,
            VarInt(self.item_entity_id),
            VarInt(self.entity_id)
        );
        writer.write_all(&buf).await
    }
}
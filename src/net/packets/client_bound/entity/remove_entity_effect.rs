use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

pub struct RemoveEntityEffect {
    entity_id: i32,
    effect_id: i8, // writes signed, reads unsigned. ig its probably never negative though.
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for RemoveEntityEffect {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x1E,
            VarInt(self.entity_id),
            self.effect_id
        );
        writer.write_all(&buf).await
    }
}
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

pub struct EntityStatus {
    entity_id: i32,
    logic_op_code: i8,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityStatus {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x19,
            self.entity_id, // not a varint
            self.logic_op_code
        );
        writer.write_all(&buf).await
    }
}
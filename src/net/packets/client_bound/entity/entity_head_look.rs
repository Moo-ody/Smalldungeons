use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

pub struct EntityHeadLook {
    entity_id: i32,
    yaw: i8,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityHeadLook {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x19,
            self.entity_id, // not a varint
            self.yaw
        );
        writer.write_all(&buf).await
    }
}
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use async_trait::async_trait;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Copy, Clone)]
pub struct CloseWindowPacket {
    pub window_id: i8,
}

#[async_trait]
impl ClientBoundPacketImpl for CloseWindowPacket {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x2e,
            self.window_id,
        );
        writer.write_all(&buf).await
    }
}
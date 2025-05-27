use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacket;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug)]
pub struct ServerInfo {
    pub status: String,
}

#[async_trait::async_trait]
impl ClientBoundPacket for ServerInfo {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x00,
            self.status
        );

        writer.write_all(&buf).await
    }
}
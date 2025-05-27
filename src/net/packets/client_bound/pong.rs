use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct Pong {
    pub client_time: i64
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for Pong {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x01,
            self.client_time
        );

        writer.write_all(&buf).await
    }
}
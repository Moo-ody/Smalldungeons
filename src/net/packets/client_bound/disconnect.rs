use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::utils::chat_component::chat_component_text::ChatComponentText;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct Disconnect {
    pub reason: ChatComponentText
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for Disconnect {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x40,
            self.reason,
        );
        writer.write_all(&buf).await
    }
}
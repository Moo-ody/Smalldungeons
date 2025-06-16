use tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::utils::chat_component::chat_component_text::ChatComponentText;

#[derive(Clone, Debug)]
pub struct PlayerListHeaderFooter {
    pub header: ChatComponentText,
    pub footer: ChatComponentText,
}

impl PlayerListHeaderFooter {}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for PlayerListHeaderFooter {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x47,
            self.header,
            self.footer,
        );
        writer.write_all(&buf).await
    }
}
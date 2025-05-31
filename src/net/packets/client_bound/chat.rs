use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::utils::chat_component::chat_component_text::ChatComponentText;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

pub const CHAT: i8 = 1;
pub const ACTION_BAR: i8 = 2;

#[derive(Debug, Clone)]
pub struct Chat {
    pub component: ChatComponentText,
    pub typ: i8,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for Chat {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x02,
            self.component,
            self.typ,
        );
        writer.write_all(&buf).await
    }
}
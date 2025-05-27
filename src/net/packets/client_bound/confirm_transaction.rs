use tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;

#[derive(Debug)]
pub struct ConfirmTransaction {
    pub window_id: i8,
    pub action_number: i16,
    pub accepted: bool,
}

impl ConfirmTransaction {
    pub fn new() -> ConfirmTransaction {
        ConfirmTransaction {
            window_id: 0i8,
            action_number: 0i16,
            accepted: false,
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for ConfirmTransaction {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x32,
            self.window_id,
            self.action_number,
            self.accepted,
        );
        writer.write_all(&buf).await
    }
}
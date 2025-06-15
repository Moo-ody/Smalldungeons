use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::items::item_stack::ItemStack;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct SetSlot {
    pub window_id: i8,
    pub slot: i16,
    pub item_stack: Option<ItemStack>,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for SetSlot {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x2f,
            self.window_id,
            self.slot,
            self.item_stack,
        );
        writer.write_all(&buf).await
    }
}
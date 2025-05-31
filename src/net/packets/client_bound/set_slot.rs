use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::items::item_stack::ItemStack;
use crate::{build_packet, print_bytes_hex};
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct SetSlot {
    pub window_id: i8,
    pub slot: i16,
    pub item_stack: ItemStack,
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

        print_bytes_hex!("Set Slot Packet", buf);
        writer.write_all(&buf).await
    }
}
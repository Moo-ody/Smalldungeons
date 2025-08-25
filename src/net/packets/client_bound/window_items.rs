use crate::net::packets::packet::{finish_packet, ClientBoundPacketImpl};
use crate::net::packets::packet_write::PacketWrite;
use crate::net::var_int::write_var_int;
use crate::server::items::item_stack::ItemStack;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct WindowItems {
    pub window_id: u8,
    pub items: Vec<Option<ItemStack>>,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for WindowItems {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut payload = Vec::new();

        write_var_int(&mut payload, 0x30);
        self.window_id.write(&mut payload);
        (self.items.len() as i16).write(&mut payload);
        for item_stack in self.items.iter() {
            item_stack.write(&mut payload);
        }

        writer.write_all(&finish_packet(payload)).await
    }
}
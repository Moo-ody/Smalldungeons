use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::var_int::VarInt;
use crate::server::items::item_stack::ItemStack;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug, Clone)]
pub struct EntityEquipment {
    pub entity_id: i32,
    pub item_slot: i16,
    pub item_stack: Option<ItemStack>,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityEquipment {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x04,
            VarInt(self.entity_id),
            self.item_slot,
            self.item_stack,
        );
        writer.write_all(&buf).await
    }
}
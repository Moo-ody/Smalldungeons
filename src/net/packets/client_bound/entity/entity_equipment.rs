use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use crate::server::items::item_stack::ItemStack;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

pub struct EntityEquipment {
    entity_id: i32,
    item_slot: i16,
    item_stack: Option<ItemStack>,
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
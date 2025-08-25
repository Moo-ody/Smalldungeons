use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::var_int::VarInt;
use crate::server::block::block_pos::BlockPos;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Copy, Clone)]
pub struct PacketBlockAction {
    pub block_pos: BlockPos,
    pub event_id: u8,
    pub event_data: u8,
    pub block_id: i32,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for PacketBlockAction {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x24,
            self.block_pos,
            self.event_id,
            self.event_data,
            VarInt(self.block_id & 4095)
        );
        writer.write_all(&buf).await
    }
}
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::var_int::VarInt;
use crate::server::block::block_pos::BlockPos;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct BlockChange {
    pub block_pos: BlockPos,
    pub block_state: u16,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for BlockChange {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x23,
            self.block_pos,
            VarInt(self.block_state as i32),
        );
        writer.write_all(&buf).await
    }
}
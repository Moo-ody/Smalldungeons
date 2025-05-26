use tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacket;
use crate::net::varint::VarInt;

pub struct Disconnect {
    pub reason: String,
}

#[async_trait::async_trait]
impl ClientBoundPacket for Disconnect {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x00,
            VarInt(self.reason.len() as i32),
            self.reason.as_bytes(),
        );
        
        writer.write_all(&buf).await
    }
}
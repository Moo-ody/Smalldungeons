use tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;

#[derive(Debug)]
pub struct KeepAlive {
    pub current_time: i32
}

impl KeepAlive {
    pub fn from_time(time: i32) -> KeepAlive {
        KeepAlive {
            current_time: time
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for KeepAlive {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> tokio::io::Result<()> {
        let buf = build_packet!(
            0x00,
            VarInt(self.current_time)
        );
        
        writer.write_all(&buf).await
    }
}
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub struct TimeUpdate {
    world_age: i64,
    world_time: i64,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for TimeUpdate {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x03,
            self.world_age,
            self.world_time,
        );
        writer.write_all(&buf).await
    }
}

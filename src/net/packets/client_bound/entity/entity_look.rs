use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

pub struct EntityLook {
    entity_id: i32,
    yaw: i8,
    pitch: i8,
    on_ground: bool,
}
#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityLook {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x16,
            VarInt(self.entity_id),
            self.yaw,
            self.pitch,
            self.on_ground
        );
        writer.write_all(&buf).await
    }
}
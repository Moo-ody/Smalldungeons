use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug)]
pub struct EntityHeadLook {
    entity_id: i32,
    yaw: i8,
}

impl EntityHeadLook {
    pub fn new(entity_id: i32, yaw: f32) -> Self {
        Self {
            entity_id,
            yaw: (yaw * 256.0 / 360.0) as i8,
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityHeadLook {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x19,
            VarInt(self.entity_id),
            self.yaw
        );
        writer.write_all(&buf).await
    }
}
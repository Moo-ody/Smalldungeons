use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

pub struct EntityRelMove {
    entity_id: i32,
    pos_x: i8,
    pos_y: i8,
    pos_z: i8,
    on_ground: bool,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityRelMove {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x15,
            VarInt(self.entity_id),
            self.pos_x,
            self.pos_y,
            self.pos_z,
            self.on_ground
        );
        writer.write_all(&buf).await
    }
}
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::packets::packet_write::PacketWrite;
use crate::net::varint::VarInt;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug, Clone)]
pub struct DestroyEntities {
    pub entity_ids: Vec<i32>,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for DestroyEntities {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let mut payload = Vec::new();
        VarInt(0x13).write(&mut payload);

        VarInt(self.entity_ids.len() as i32).write(&mut payload);
        for id in self.entity_ids.iter() {
            VarInt(*id).write(&mut payload);
        }

        let mut buf = Vec::new();
        VarInt(payload.len() as i32).write(&mut buf);
        buf.extend_from_slice(&payload);

        writer.write_all(&buf).await
    }
}
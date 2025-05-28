use crate::net::packets::packet::{ClientBoundPacketImpl, PacketWrite};
use crate::net::varint::VarInt;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct DestroyEntities {
    pub(crate) entity_ids: Vec<i32>,
}

impl DestroyEntities {}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for DestroyEntities {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
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
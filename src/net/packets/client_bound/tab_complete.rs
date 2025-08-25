use crate::net::packets::packet::{finish_packet, ClientBoundPacketImpl};
use crate::net::packets::packet_write::PacketWrite;
use crate::net::var_int::VarInt;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct TabComplete {
    pub matches: Vec<String>,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for TabComplete {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut payload = Vec::new();
        VarInt(0x3A).write(&mut payload);
        VarInt(self.matches.len() as i32).write(&mut payload);
        for str in &self.matches {
            str.write(&mut payload);
        }

        writer.write_all(&finish_packet(payload)).await
    }
}
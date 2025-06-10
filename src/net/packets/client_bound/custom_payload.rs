use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::packets::packet_write::PacketWrite;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct CustomPayload {
    // must not be longer than 20
    pub channel: String,
    pub data: Vec<u8>,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for CustomPayload {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x3f,
            self.channel,
            self.data,
        );
        writer.write_all(&buf).await
    }
}

impl PacketWrite for Vec<u8> {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self);
    }
}
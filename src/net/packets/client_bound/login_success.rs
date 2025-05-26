use tokio::io::{AsyncWrite, AsyncWriteExt, Result};
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacket;
use crate::net::varint::VarInt;

#[derive(Debug)]
pub struct LoginSuccess {
    pub uuid: String,
    pub name: String,
}

#[async_trait::async_trait]
impl ClientBoundPacket for LoginSuccess {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x02,
            VarInt(self.uuid.len() as i32),
            self.uuid.as_bytes(),
            VarInt(self.name.len() as i32),
            self.name.as_bytes(),
        );

        let hex_string: String = buf.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(" ");

        println!("Raw bytes [{}]: {}", buf.len(), hex_string);
        
        writer.write_all(&buf).await
    }
}
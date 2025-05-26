use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacket;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

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
            self.uuid,
            self.name,
        );

        let hex_string: String = buf.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(" ");

        println!("Raw bytes [{}]: {}", buf.len(), hex_string);
        
        writer.write_all(&buf).await
    }
}
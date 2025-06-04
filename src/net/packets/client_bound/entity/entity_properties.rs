use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::var_int::VarInt;
use crate::server::entity::attributes::Attributes;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

pub struct EntityProperties {
    pub entity_id: i32,
    pub properties: Attributes,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityProperties {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x20,
            VarInt(self.entity_id),
            self.properties,
        );
        writer.write_all(&buf).await
    }
}
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

pub struct EntityEffect {
    entity_id: i32,
    effect_id: i8,
    amplifier: i8,
    duration: i32,
    hide_particles: bool,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityEffect {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x1D,
            VarInt(self.entity_id),
            self.effect_id,
            self.amplifier,
            VarInt(self.duration),
            self.hide_particles as u8
        );
        writer.write_all(&buf).await
    }
}
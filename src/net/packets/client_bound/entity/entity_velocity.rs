use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct EntityVelocity {
    pub entity_id: i32,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
}

// impl EntityVelocity {
//     pub fn 
// }

#[async_trait::async_trait]
impl ClientBoundPacketImpl for EntityVelocity {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let motion_clamp = 3.9;
        let buf = build_packet!(
            0x12,
            VarInt(self.entity_id),
            (self.motion_x.clamp(-motion_clamp, motion_clamp) * 8000.0) as i16,
            (self.motion_y.clamp(-motion_clamp, motion_clamp) * 8000.0) as i16,
            (self.motion_z.clamp(-motion_clamp, motion_clamp) * 8000.0) as i16,
        );
        writer.write_all(&buf).await
    }
}
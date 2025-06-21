use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::utils::sounds::Sounds;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct SoundEffect {
    pub sounds: Sounds,
    pub volume: f32,
    pub pitch: f32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for SoundEffect {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x29,
            self.sounds.id(),
            (self.x * 8.0) as i32,
            (self.y * 8.0) as i32,
            (self.z * 8.0) as i32,
            self.volume,
            (self.pitch * 63.0).clamp(0.0, 255.0) as u8
        );
        writer.write_all(&buf).await
    }
}
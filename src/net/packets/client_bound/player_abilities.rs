use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::build_packet;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct PlayerAbilities {
    pub invulnerable: bool,
    pub flying: bool,
    pub allow_flying: bool,
    pub creative_mode: bool,
    pub flying_speed: f32,
    pub walking_speed: f32,
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for PlayerAbilities {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut byte: i8 = 0;
        if self.invulnerable {
            byte |= 1;
        }
        if self.flying {
            byte |= 2;
        }
        if self.allow_flying {
            byte |= 4;
        }
        if self.creative_mode {
            byte |= 8;
        }

        let buf = build_packet!(
            0x39,
            byte,
            self.flying_speed,
            self.walking_speed
        );

        writer.write_all(&buf).await
    }
}
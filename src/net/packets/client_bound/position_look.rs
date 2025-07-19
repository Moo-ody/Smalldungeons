use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::player::player::Player;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug, Clone, Copy)]
pub struct PositionLook {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: u8,
}

impl PositionLook {
    pub fn from_player(player: &Player) -> Self {
        Self {
            x: player.position.x,
            y: player.position.y,
            z: player.position.z,
            yaw: player.yaw,
            pitch: player.pitch,
            flags: 0,
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for PositionLook {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x08,
            self.x,
            self.y,
            self.z,
            self.yaw,
            self.pitch,
            self.flags,
            // VarInt(self.teleport_id)
        );

        writer.write_all(&buf).await
    }
}
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::player::Player;
use tokio::io::{AsyncWrite, AsyncWriteExt, Result};

#[derive(Debug)]
pub struct PositionLook {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: u8,
    // pub teleport_id: i32,
}

impl PositionLook {
    pub(crate) fn from_player(player: &Player) -> PositionLook {
        PositionLook {
            x: player.entity.pos.x,
            y: player.entity.pos.y,
            z: player.entity.pos.z,
            yaw: player.entity.yaw,
            pitch: player.entity.pitch,
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
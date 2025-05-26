use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacket;
use crate::server::entity::player_entity::PlayerEntity;
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
    pub(crate) fn from_player(player: &PlayerEntity) -> PositionLook {
        PositionLook {
            x: player.pos_x,
            y: player.pos_y,
            z: player.pos_z,
            yaw: player.yaw,
            pitch: player.pitch,
            flags: 0,
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacket for PositionLook {
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
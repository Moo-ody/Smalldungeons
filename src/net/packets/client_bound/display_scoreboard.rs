use tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::server::utils::scoreboard::SizedString;

#[derive(Clone, Debug)]
pub struct DisplayScoreboard {
    pub position: i8,
    pub score_name: SizedString<16>,
}

pub const SIDEBAR: i8 = 1;
pub const BELOW_NAME: i8 = 2;

impl DisplayScoreboard {}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for DisplayScoreboard {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x3D,
            self.position,
            self.score_name,
        );
        writer.write_all(&buf).await
    }
}
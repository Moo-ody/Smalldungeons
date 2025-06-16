use crate::net::packets::packet::{finish_packet, ClientBoundPacketImpl};
use crate::net::packets::packet_write::PacketWrite;
use crate::net::var_int::VarInt;
use crate::server::utils::scoreboard::sized_string::SizedString;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct Teams {
    pub name: SizedString<16>,
    pub display_name: SizedString<32>,
    pub prefix: SizedString<16>,
    pub suffix: SizedString<16>,
    pub name_tag_visibility: SizedString<32>,
    pub color: i8,
    pub players: Vec<SizedString<40>>,
    pub action: i8,
    pub friendly_flags: i8,
}

pub const CREATE_TEAM: i8 = 0;
pub const REMOVE_TEAM: i8 = 1;
pub const UPDATE_TEAM: i8 = 2;
pub const ADD_PLAYER: i8 = 3;
pub const REMOVE_PLAYER: i8 = 4;

impl Teams {}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for Teams {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut payload = Vec::new();
        VarInt(0x3E).write(&mut payload);
        self.name.write(&mut payload);
        self.action.write(&mut payload);

        if self.action == 0 || self.action == 2 {
            self.display_name.write(&mut payload);
            self.prefix.write(&mut payload);
            self.suffix.write(&mut payload);
            self.friendly_flags.write(&mut payload);
            self.name_tag_visibility.write(&mut payload);
            self.color.write(&mut payload);
        }

        if self.action == 0 || self.action == 3 || self.action == 4 {
            VarInt(self.players.len() as i32).write(&mut payload);
            for player in self.players.iter() {
                player.write(&mut payload);
            }
        }

        writer.write_all(&finish_packet(payload)).await
    }
}
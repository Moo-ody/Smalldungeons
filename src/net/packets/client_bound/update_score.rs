use crate::id_enum;
use crate::net::packets::packet::{finish_packet, ClientBoundPacketImpl};
use crate::net::packets::packet_write::PacketWrite;
use crate::net::var_int::VarInt;
use crate::server::utils::scoreboard::sized_string::SizedString;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct UpdateScore {
    pub name: SizedString<40>,
    pub objective: SizedString<16>,
    pub value: i32,
    pub action: UpdateScoreAction,
}

impl UpdateScore {}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for UpdateScore {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut payload = Vec::new();
        VarInt(0x3C).write(&mut payload);
        self.name.write(&mut payload);
        VarInt(self.action.id()).write(&mut payload);
        self.objective.write(&mut payload);

        if self.action != UpdateScoreAction::Remove {
            VarInt(self.value).write(&mut payload);
        }

        writer.write_all(&finish_packet(payload)).await
    }
}

id_enum!(
    pub enum UpdateScoreAction: i32 {
        Change(0),
        Remove(1),
    }
);
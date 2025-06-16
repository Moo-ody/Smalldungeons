use crate::id_enum;
use crate::net::packets::packet::{finish_packet, ClientBoundPacketImpl};
use crate::net::packets::packet_write::PacketWrite;
use crate::net::var_int::VarInt;
use crate::server::utils::scoreboard::sized_string::SizedString;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Clone, Debug)]
pub struct ScoreboardObjective {
    pub objective_name: SizedString<16>,
    pub objective_value: SizedString<32>,
    pub typ: ScoreboardRenderType,
    pub mode: i8,
}

pub const ADD_OBJECTIVE: i8 = 0;
pub const UPDATE_NAME: i8 = 2;


#[async_trait::async_trait]
impl ClientBoundPacketImpl for ScoreboardObjective {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut payload = Vec::new();
        VarInt(0x3B).write(&mut payload);
        self.objective_name.write(&mut payload);
        self.mode.write(&mut payload);
        if self.mode == ADD_OBJECTIVE || self.mode == 2 {
            self.objective_value.write(&mut payload);
            self.typ.id().write(&mut payload);
        }

        writer.write_all(&finish_packet(payload)).await
    }
}

id_enum! {
    pub enum ScoreboardRenderType: &'static str {
        Integer("integer"),
        Hearts("hearts"),
    }
}
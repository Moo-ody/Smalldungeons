use tokio::io::{AsyncWrite, AsyncWriteExt, Result};
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacket;
use crate::server::entity::player_entity::PlayerEntity;

#[derive(Debug)]
pub struct JoinGame {
    pub entity_id: i32,
    pub gamemode: u8,
    pub dimension: u8,
    pub difficulty: u8,
    pub max_players: u8,
    pub level_type: &'static str,
    pub reduced_debug_info: u8,
}

impl JoinGame {
    pub fn from_player(player: &PlayerEntity) -> JoinGame {
        JoinGame {
            entity_id: player.entity_id as i32,
            gamemode: 1,
            dimension: 0,
            difficulty: 0,
            max_players: 0,
            level_type: "",
            reduced_debug_info: 0,
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacket for JoinGame {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> Result<()> {
        let buf = build_packet!(
            0x01,
            self.entity_id,
            self.gamemode,
            self.dimension,
            self.difficulty,
            self.max_players,
            self.level_type,
            self.reduced_debug_info,
        );

        let hex_string: String = buf.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(" ");

        println!("Raw bytes [{}]: {}", buf.len(), hex_string);
        
        writer.write_all(&buf).await
    }
}
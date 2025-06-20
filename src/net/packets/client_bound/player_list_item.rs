use crate::net::packets::packet::{finish_packet, ClientBoundPacketImpl};
use crate::net::packets::packet_write::PacketWrite;
use crate::net::var_int::VarInt;
use crate::server::utils::player_list::player_profile::PlayerData;
use crate::id_enum;
use std::collections::HashMap;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct PlayerListItem {
    action: PlayerListAction,
    players: Vec<PlayerData>,
}

impl PlayerListItem {
    pub fn new(action: PlayerListAction, players: &[PlayerData]) -> Self {
        Self {
            action,
            players: players.to_vec(),
        }
    }

    pub fn init_packet(player_list: &HashMap<i32, PlayerData>) -> Self {
        Self {
            action: PlayerListAction::AddPlayer,
            players: player_list.values().cloned().collect::<Vec<PlayerData>>(),
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for PlayerListItem {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        println!("{:?}", self);
        let mut payload = Vec::new();
        VarInt(0x38).write(&mut payload);

        VarInt(self.action.id()).write(&mut payload);
        VarInt(self.players.len() as i32).write(&mut payload);

        for player in self.players.iter() {
            match self.action {
                PlayerListAction::AddPlayer => {
                    player.profile.id.write(&mut payload);
                    player.profile.name.write(&mut payload);
                    VarInt(player.profile.properties.len() as i32).write(&mut payload);
                    for (key, property) in player.profile.properties.iter() {
                        key.write(&mut payload);
                        property.value.write(&mut payload);
                        if let Some(signature) = &property.signature {
                            true.write(&mut payload);
                            signature.write(&mut payload);
                        } else {
                            false.write(&mut payload);
                        }
                    }

                    VarInt(player.game_mode.id()).write(&mut payload);
                    VarInt(player.ping).write(&mut payload);
                }
                PlayerListAction::UpdateGameMode => {
                    player.profile.id.write(&mut payload);
                    VarInt(player.game_mode.id()).write(&mut payload);
                }
                PlayerListAction::UpdateLatency => {
                    player.profile.id.write(&mut payload);
                    VarInt(player.ping).write(&mut payload);
                }
                PlayerListAction::UpdateDisplayName => {
                    player.profile.id.write(&mut payload);
                }
                PlayerListAction::RemovePlayer => {
                    player.profile.id.write(&mut payload);
                }
            }

            if self.action == PlayerListAction::AddPlayer || self.action == PlayerListAction::UpdateDisplayName {
                if let Some(display_name) = &player.display_name {
                    true.write(&mut payload);
                    display_name.write(&mut payload);
                } else {
                    false.write(&mut payload);
                }
            }
        }

        writer.write_all(&finish_packet(payload)).await
    }
}


/// idk how to do ordinal with enums in rust atm and chatgpt sucks
id_enum! {
    pub enum PlayerListAction: i32 {
        AddPlayer(0),
        UpdateGameMode(1),
        UpdateLatency(2),
        UpdateDisplayName(3),
        RemovePlayer(4)
    }
}
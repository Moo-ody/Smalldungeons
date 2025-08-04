use crate::id_enum;
use crate::server::player::player::{GameProfile, GameProfileProperty};
use crate::server::utils::chat_component::chat_component_text::{ChatComponentText, ChatComponentTextBuilder};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PlayerData {
    pub ping: i32,
    pub game_mode: GameType,
    pub profile: GameProfile,
    pub display_name: Option<ChatComponentText>,
}

pub const GRAY: &str = "eyJ0aW1lc3RhbXAiOjE0NjU2NTg1NjkzNDQsInByb2ZpbGVJZCI6ImIwZDRiMjhiYzFkNzQ4ODlhZjBlODY2MWNlZTk2YWFiIiwicHJvZmlsZU5hbWUiOiJJbnZlbnRpdmVHYW1lcyIsInNpZ25hdHVyZVJlcXVpcmVkIjp0cnVlLCJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvYTdmNWZkMjNhYWZiYjlhYjRmZDE0ZmQwODIzYWVhM2E5ZmJkZmU2ZmIyZGUwZjdlZjMzZDBmNGI0YWI3YSJ9fX0=";
pub const GRAY_SIG: &str = "Ntyez/Ut7Ht7cmrgPK3dJ/MvIEyQt3jy2c6ACx9sfxB4AKNzyE6fsjffRWwXuObEiPGwYm6iIl2eWuMSvlWljXBg5XJL/ql2Xse1yXImfGfUmedltUhoQuAxCPjogubcapAdGsIr4W5tVrYoTnMgjz7zyc0gHafCClEQBm8A+C5FIfFsAkWUdmzqrEPjbzlqt6qUl0m1sAdskgR0DgCg2pZk5djL+CzPiUgZv1rqXO4Mg5zXr5DSpeEgk5AMInEuxCRemL096cp0fKwf99FEUOiJ2Rq6J9azZVnyjoQy5sZv1RD3+24ib9uwZoJFWakjtTJnWEcUrFsFrr3p5q1Q1prCvbOKS+xD7pMWUbCVz4WuS14e0PtjF5IpDFCehwveREofru7uxAeD4lP13XTdAuWvBvPox1gKvnax8OsnQO64F4nadGm3WmWas6JYzGgOkIRwiKGTxJDIQcAmRXgn8qDx8if5EVnI3/RLIayo06FPXQGmU1p6CCoAy7prGkVeHZsoKuvRGVj3PN70aoJialHzKKpW1OYgSKQXuTOaaWjQN3xbXVrOKaEEAxglWR7drfVPM1szBpcwnYL7hIKejTiHbfZj6mPwFDf0iy1YVcY0iDeCJogf50Z5dWJrwNw6IsQCLuig7BTWAcP7iCCl73UuJRrYndeN7tUIQbtRqfM=";

impl PlayerData {
    pub fn new(profile: GameProfile) -> Self {
        Self {
            ping: 1,
            game_mode: GameType::Creative,
            profile,
            display_name: Some(ChatComponentTextBuilder::new("").build()),
        }
    }

    pub fn with_text(text: ChatComponentText) -> Self {
        Self {
            ping: 1,
            game_mode: GameType::Creative,
            profile: GameProfile {
                uuid: Uuid::new_v4(),
                username: "default".into(),
                properties: HashMap::from(
                    [(
                        "textures".to_owned(),
                        GameProfileProperty {
                            value: GRAY.to_owned(),
                            signature: GRAY_SIG.to_owned().into(),
                        }
                    )]
                ),
            },
            display_name: Some(text),
        }
    }
}

id_enum! {
    pub enum GameType: i32 {
        NotSet(-1),
        Survival(0),
        Creative(1),
        Adventure(2),
        Spectator(3)
    }
}
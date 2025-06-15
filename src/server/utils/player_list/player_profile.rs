use uuid::Uuid;
use crate::id_enum;
use crate::server::utils::chat_component::chat_component_text::ChatComponentText;

#[derive(Debug, Clone)]
pub struct PlayerData {
    pub ping: i32,
    pub game_mode: GameType,
    pub profile: GameProfile,
    pub display_name: Option<ChatComponentText>,
}

impl PlayerData {
    pub fn new(profile: GameProfile) -> Self {
        Self {
            ping: 0,
            game_mode: GameType::Survival,
            profile,
            display_name: None,
        }
    }

    pub fn with_text(text: ChatComponentText) -> Self {
        Self {
            ping: 0,
            game_mode: GameType::Survival,
            profile: GameProfile::new(), // todo: get what hypixel actually sends here
            display_name: Some(text),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameProfile {
    pub id: Uuid,
    pub name: String,
}

impl GameProfile {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "ignorefornow".to_owned(),
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
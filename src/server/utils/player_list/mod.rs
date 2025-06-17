pub mod player_profile;
pub mod header;
pub mod footer;

use crate::server::utils::chat_component::chat_component_text::ChatComponentText;
use crate::server::utils::player_list::player_profile::PlayerData;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

// we may want a way to send lines without needing to populate them, otherwise we can just use a vec instead of hashmap probably.
// this should be per player maybe? not sure.
// this could also just use like 90% of the scoreboard backend without the teams packet nonsense, since it functions very similarly.
pub struct PlayerList {
    tab_list: HashMap<i32, PlayerData>,
}

impl PlayerList {
    pub fn new() -> Self {
        Self {
            tab_list: HashMap::new()
        }
    }

    pub fn tab_list(&self) -> &HashMap<i32, PlayerData> {
        &self.tab_list
    }

    pub fn set_line(&mut self, line: i32, player_data: PlayerData) {
        self.tab_list.insert(line, player_data);
    }

    pub fn update_text(&mut self, line: i32, text: ChatComponentText) {
        match self.tab_list.entry(line) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().display_name = Some(text);
            }
            Entry::Vacant(entry) => {
                entry.insert(PlayerData::with_text(text));
            }
        };
    }

    pub fn get_line(&self, line: i32) -> Option<&PlayerData> {
        self.tab_list.get(&line)
    }
}
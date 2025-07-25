pub mod player_profile;
pub mod header;
pub mod footer;

use crate::net::packets::protocol::clientbound::PlayerListItem;
use crate::net::var_int::VarInt;
use crate::server::utils::chat_component::chat_component_text::ChatComponentText;
use crate::server::utils::player_list::player_profile::{GameProfile, PlayerData};
use std::array;
use std::collections::HashSet;
use std::rc::Rc;

pub struct PlayerList {
    lines: [PlayerData; 80],
    updated: HashSet<usize>,
}

impl PlayerList {
    pub fn new() -> Self {
        Self {
            lines: generate_default_lines(), // we might want to update this for the skull textures next to names when they matter
            updated: HashSet::new(),
        }
    }

    pub fn set_line(&mut self, index: usize, text: ChatComponentText) {
        assert!(index < self.lines.len(), "Attempted to set player list line {} but there are only {} lines", index, self.lines.len());
        let line = &mut self.lines[index];
        line.display_name = Some(text);

        self.updated.insert(index);
    }

    pub fn get_packet(&mut self) -> Option<PlayerListItem> {
        if self.updated.is_empty() {
            return None;
        }

        let lines = self.updated.drain().map(|index| {
            self.lines[index].clone()
        }).collect::<Vec<_>>();

        Some(PlayerListItem {
            action: VarInt(3),
            players: Rc::from(lines),
        })
    }

    pub fn new_packet(&mut self) -> PlayerListItem {
        PlayerListItem {
            action: VarInt(0),
            players: Rc::from(self.lines.clone()), // i dont think rc doesnt anything
        }
    }
}

// this is what hypixel does to force alphabetical order. We should be able to change this however we want as long as it maintains order.
fn generate_default_lines<const N: usize>() -> [PlayerData; N] {
    array::from_fn(|i| {
        let left = index_to_letter(i / 26);
        let right = index_to_letter(i % 26);
        PlayerData::new(GameProfile::new(format!("!{}-{}", left, right)))
    })
}

const fn index_to_letter(n: usize) -> char {
    (b'a' + (n % 26) as u8) as char
}
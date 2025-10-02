use std::collections::HashMap;
use crate::net::protocol::play::clientbound::SoundEffect;
use crate::net::protocol::play::serverbound::ClickWindow;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::player::terminals::panes::Panes;
use crate::server::utils::sounds::Sounds;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerminalType {
    Panes
}

pub(crate) trait Term {
    fn click_slot(terminal: &mut Terminal, player: &mut Player, slot: usize) -> bool;
    fn create() -> (Vec<Option<ItemStack>>, HashMap<i8, i8>);
    fn check(terminal: &mut Terminal) -> bool;
}



#[derive(Debug)]
pub struct Terminal {
    pub size: i8,
    pub items: Vec<Option<ItemStack>>,
    pub typ: TerminalType,
    pub solution: HashMap<i8, i8> // using second arg as an int for rubix and numbers
}

impl Terminal {
    pub fn new(typ: TerminalType) -> Terminal {
        let pair = Panes::create();
        Terminal {
            size: 45,
            items: pair.0,
            typ,
            solution: pair.1
        }
    }

    pub fn set_slot(&mut self, item: ItemStack, index: usize) {
        if index >= self.size as usize {
            return;
        }
        self.items[index] = Option::from(item)
    }

    pub fn get_slot_cloned(&self, slot: usize) -> Option<ItemStack> {
        self.items.get(slot).cloned().unwrap_or_else(|| None)
    }

    pub fn get_contents(&self) -> Vec<Option<ItemStack>> {
        self.items.clone()
    }

    pub fn click_slot(
        &mut self,
        packet: &ClickWindow,
        player: &mut Player
    ) -> bool {
        let slot = packet.slot_id as usize;
        match self.typ {
            TerminalType::Panes => {
                Panes::click_slot(self, player, slot)
            }
        }
    }
    pub fn play_sound(&self, player: &mut Player, sound: &str) {
        player.write_packet(&SoundEffect {
            sound,
            volume: 1.0,
            pitch: 1.0,
            pos_x: player.position.x,
            pos_y: player.position.y,
            pos_z: player.position.z,
        });
    }
}
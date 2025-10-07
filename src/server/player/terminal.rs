use std::collections::HashMap;
use crate::net::protocol::play::clientbound::SoundEffect;
use crate::net::protocol::play::serverbound::{ClickMode, ClickWindow};
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::player::terminals::order::Order;
use crate::server::player::terminals::panes::Panes;
use crate::server::player::terminals::rubix::Rubix;
use crate::server::player::terminals::select::Select;
use crate::server::player::terminals::starts_with::StartsWith;
use crate::server::utils::nbt::nbt::NBT;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerminalType {
    Melody,
    Order,
    Panes,
    Rubix,
    Select,
    StartsWith
}

pub(crate) trait Term {
    fn click_slot(terminal: &mut Terminal, player: &mut Player, slot: usize, packet: &ClickWindow) -> bool;
    fn create(rand: i16) -> (Vec<Option<ItemStack>>, HashMap<i8, i8>);
    fn check(terminal: &Terminal) -> bool;
}



#[derive(Debug)]
pub struct Terminal {
    pub size: i8,
    pub items: Vec<Option<ItemStack>>,
    pub typ: TerminalType,
    pub solution: HashMap<i8, i8>, // using second arg as an int for rubix and numbers
    pub rand: i16
}

impl Terminal {
    pub fn new(typ: TerminalType, rand: i16) -> Terminal {
        let pair;
        let size;
        match typ {
            TerminalType::Order => {
                pair = Order::create(rand);
                size = 4*9;
            }
            TerminalType::Panes => {
                pair = Panes::create(rand);
                size = 5*9;
            }
            TerminalType::Rubix => {
                pair = Rubix::create(rand);
                size = 5*9;
            }
            TerminalType::Select => {
                pair = Select::create(rand);
                size = 6*9;
            }
            TerminalType::StartsWith => {
                pair = StartsWith::create(rand);
                size = 6*9;
            }
            _ => {
                pair = Panes::create(rand);
                size = 5*9;
            }
        }

        Terminal {
            size,
            items: pair.0,
            typ,
            solution: pair.1,
            rand
        }
    }

    pub fn set_slot(&mut self, item: ItemStack, index: usize) {
        if index >= self.size as usize {
            return;
        }
        self.items[index] = Option::from(item)
    }

    pub fn get_contents(&self) -> Vec<Option<ItemStack>> {
        self.items.clone()
    }

    pub fn get_slot_cloned(&self, slot: usize) -> Option<ItemStack> {
        self.items.get(slot).cloned().unwrap_or_else(|| None)
    }

    pub fn click_slot(
        &mut self,
        packet: &ClickWindow,
        player: &mut Player
    ) -> bool {
        let slot = packet.slot_id as usize;
        match self.typ {
            TerminalType::Order => {
                Order::click_slot(self, player, slot, packet)
            }
            TerminalType::Panes => {
                Panes::click_slot(self, player, slot, packet)
            }
            TerminalType::Rubix => {
                Rubix::click_slot(self, player, slot, packet)
            }
            TerminalType::Select => {
                Select::click_slot(self, player, slot, packet)
            }
            TerminalType::StartsWith => {
                StartsWith::click_slot(self, player, slot, packet)
            }
            _ => { false }
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
pub fn pane(meta: i16, name: &str) -> ItemStack {
    ItemStack {
        item: 160,
        stack_size: 1,
        metadata: meta,
        tag_compound: Some(NBT::with_nodes(vec![
            NBT::compound("display", vec![
                NBT::string("Name", name)
            ])
        ])),
    }
}
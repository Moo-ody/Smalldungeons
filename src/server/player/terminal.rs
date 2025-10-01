use crate::net::protocol::play::serverbound::ClickWindow;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::terminals::panes::Panes;

#[derive(Debug)]
pub enum TerminalType {
    Panes
}

pub(crate) trait Term {
    fn click_slot(terminal: &mut Terminal, slot: usize) -> bool;
    fn create() -> Vec<Option<ItemStack>>;
    fn check(terminal: &mut Terminal) -> bool;
}



#[derive(Debug)]
pub struct Terminal {
    pub size: i8, // what is ts, where my int at :sob:
    pub items: Vec<Option<ItemStack>>,
    pub _type: TerminalType
}

impl Terminal {
    pub fn new( _type: TerminalType) -> Terminal {
        Terminal {
            size: 45,
            items: Panes::create(),
            _type
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
        packet: &ClickWindow
    ) -> bool {
        let slot = packet.slot_id as usize;
        match self._type { // is it obv how much i wanna use proper objects
            TerminalType::Panes => {
                Panes::click_slot(self, slot)
            }
        }
    }
}
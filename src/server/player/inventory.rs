use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::SetSlot;
use crate::net::protocol::play::serverbound::{ClickMode, ClickWindow};
use crate::server::items::item_stack::ItemStack;
use crate::server::items::Item;

#[derive(Default, Debug, Clone)]
pub enum ItemSlot {
    #[default]
    Empty,
    Filled(Item, u8), // Item and stack size
}

impl ItemSlot {
    pub fn get_item_stack(&self) -> Option<ItemStack> {
        if let ItemSlot::Filled(item, stack_size) = self {
            let mut item_stack = item.get_item_stack();
            item_stack.stack_size = *stack_size as i8;
            Some(item_stack)
        } else {
            None
        }
    }
}

// this is currently a prototype/example idk
// im not sure how i want to handle items
#[derive(Debug)]
pub struct Inventory {
    /// list of all slots
    pub items: [ItemSlot; 45],
    pub(crate) dragged_item: ItemSlot,
}

impl Inventory {

    /// creates an empty inventory
    pub fn empty() -> Inventory {
        const EMPTY: ItemSlot = ItemSlot::Empty;
        Inventory { items: [EMPTY; 45], dragged_item: EMPTY  }
    }

    pub fn set_slot(&mut self, item_slot: ItemSlot, index: usize) {
        if index >= 45 {
            return;
        }
        self.items[index] = item_slot
    }

    pub fn get_slot_cloned(&self, slot: usize) -> ItemSlot {
        self.items.get(slot).cloned().unwrap_or_else(|| ItemSlot::Empty)
    }

    pub fn get_hotbar_slot(&self, index: usize) -> Option<ItemSlot> {
        let index = index + 36;
        if index >= 36 && index <= 44 {
            return self.items.get(index).cloned();
        }
        None
    }
    
    pub fn click_slot(
        &mut self,
        packet: &ClickWindow,
        packet_buffer: &mut PacketBuffer
    ) -> bool {
        match packet.mode {
            ClickMode::NormalClick => {
                // if we ever have stackable items. this will need fixing
                if packet.slot_id < 0 { 
                    packet_buffer.write_packet(&SetSlot {
                        window_id: -1,
                        slot: 0,
                        item_stack: self.dragged_item.get_item_stack(),
                    })
                } else {
                    let slot = packet.slot_id as usize;
                    if is_valid_range(slot) {
                        let item = self.get_slot_cloned(slot);
                        self.set_slot(self.dragged_item.clone(), slot);
                        self.dragged_item = item;
                    }
                }
            }
            ClickMode::ShiftClick => {
                let slot = packet.slot_id as usize;
                if is_valid_range(slot) {
                    let clicked_stack = self.get_slot_cloned(slot);
                    let range = if slot >= 36 { 9..36 } else { 36..45 };
    
                    for index in range {
                        let item = self.get_slot_cloned(index);
                        if let ItemSlot::Empty = &item {
                            self.set_slot(clicked_stack, index);
                            self.set_slot(item, slot);
                            break;
                        }
                    }
                }
            }
            ClickMode::NumberKey => {
                let slot = packet.slot_id as usize;
                let button = packet.used_button as usize;
    
                if is_valid_range(slot) && button <= 9 {
    
                    // this is what hypixel does, that allows ghost pickaxes
                    let to_slot = 36 + button;
                    let item = self.get_slot_cloned(slot);
    
                    if to_slot == slot {
                        packet_buffer.write_packet(&SetSlot {
                            window_id: 0,
                            slot: slot as i16,
                            item_stack: item.get_item_stack(),
                        })
                    } else {
                        let item_to = self.get_slot_cloned(to_slot);
                        self.set_slot(item, to_slot);
                        self.set_slot(item_to, slot);
                    }
                }
            }
            ClickMode::MiddleClick => {
                
            }
            ClickMode::Drop => {
                let slot = packet.slot_id as usize;
                if is_valid_range(slot) {
                    packet_buffer.write_packet(&SetSlot {
                        window_id: 0,
                        slot: packet.slot_id,
                        item_stack: self.get_slot_cloned(slot).get_item_stack(),
                    })
                }
            }
            ClickMode::Drag => {}
            ClickMode::DoubleClick => {}
        }
        false
    }
}

fn is_valid_range(index: usize) -> bool {
    index >= 9 && index <= 43
}

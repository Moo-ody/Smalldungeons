use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::packets::server_bound::click_window::{ClickMode, ClickWindow};
use crate::server::items::item_stack::ItemStack;
use crate::server::items::Item;
use crate::server::player::player::ClientId;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Default, Debug, Clone)]
pub enum ItemSlot {
    #[default]
    Empty,
    Filled(Item, /*ItemStack*/),
}

impl ItemSlot {
    pub fn get_item_stack(&self) -> Option<ItemStack> {
        if let ItemSlot::Filled(item) = self {
            Some(item.get_item_stack())
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

    // caveats:
    // customizing armor would require implementation
    // stacked items would also require implementation
    // also currently almost no sync detection with confirm transaction packets and such.
    pub fn click_slot(
       &mut self,
       packet: &ClickWindow,
       client_id: &ClientId,
       network_tx: &UnboundedSender<NetworkThreadMessage>,
    ) -> anyhow::Result<bool> {
        match packet.mode {
            ClickMode::NormalClick => {
                // this currently doesn't implement different buttons.
                let slot = packet.slot_id;
                if slot < 0 {
                    // SetSlot {
                    //     window_id: -1,
                    //     slot: 0,
                    //     item_stack: self.dragged_item.clone().get_item_stack(),
                    // }.send_packet(*client_id, network_tx)?;
                } else if is_valid_range(slot as usize) {
                    let item = self.get_slot_cloned(slot as usize);
                    self.set_slot(self.dragged_item.clone(), slot as usize);
                    self.dragged_item = item;
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
                        // SetSlot {
                        //     window_id: 0,
                        //     slot: slot as i16,
                        //     item_stack: item.get_item_stack(),
                        // }.send_packet(*client_id, network_tx)?;
                    } else {
                        let item_to = self.get_slot_cloned(to_slot);
                        self.set_slot(item, to_slot);
                        self.set_slot(item_to, slot);
                    }
                }
            }
            ClickMode::Drop => {
                let slot = packet.slot_id as usize;
                if is_valid_range(slot) {
                    // SetSlot {
                    //     window_id: 0,
                    //     slot: packet.slot_id,
                    //     item_stack: self.get_slot_cloned(slot).get_item_stack(),
                    // }.send_packet(*client_id, network_tx)?;
                }
            }
            ClickMode::DoubleClick => {}
            _ => return Ok(true)
        }
        Ok(false)
    }
}

fn is_valid_range(index: usize) -> bool {
    index >= 9 && index <= 43
}

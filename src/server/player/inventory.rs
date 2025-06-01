use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::window_items::WindowItems;
use crate::net::packets::packet::SendPacket;
use crate::server::items::item_stack::ItemStack;
use crate::server::items::Item;
use crate::server::player::Player;
use tokio::sync::mpsc::UnboundedSender;

// this is currently a prototype/example idk
// im not sure how i want to handle items
#[derive(Debug)]
pub struct Inventory {
    /// list of all slots
    pub items: [ItemSlot; 45],
}

#[derive(Default, Debug, Clone)]
pub enum ItemSlot {
    #[default]
    Empty,
    Filled(&'static Item, ItemStack),
}

impl Inventory {

    /// creates an empty inventory
    pub fn empty() -> Inventory {
        const EMPTY: ItemSlot = ItemSlot::Empty;
        Inventory { items: [EMPTY; 45] }
    }

    pub fn set_slot(&mut self, item_slot: ItemSlot, index: usize) {
        if index >= 45 {
            return;
        }
        self.items[index] = item_slot
    }

    pub fn get_hotbar_slot(&self, index: usize) -> Option<ItemSlot> {
        let index = index + 27;
        if index >= 25 && index <= 36 {
            return self.items.get(index).cloned();
        }
        None
    }

    pub fn sync(
        &self,
        player: &Player,
        network_tx: &UnboundedSender<NetworkMessage>
    ) -> anyhow::Result<()> {
        let mut window_items: Vec<Option<ItemStack>> = Vec::with_capacity(45);
        for _ in 0..9 {
            window_items.push(None);
            // window_items.push(Some(ItemStack {
            //     item: 277,
            //     stack_size: 1,
            //     metadata: 0,
            //     tag_compound: Some(NBT::with_nodes(vec![
            //         NBT::compound("display", vec![
            //             NBT::string("Name", "AOTV")
            //         ])
            //     ])),
            // }));
        }
        for item in &self.items {
            if let ItemSlot::Filled(_, item) = item {
                window_items.push(Some(item.clone()));
            } else {
                window_items.push(None);
            }
        }

        let packet = WindowItems {
            window_id: 0,
            items: window_items,
        };
        packet.send_packet(player.client_id, network_tx)?;
        Ok(())
    }
}


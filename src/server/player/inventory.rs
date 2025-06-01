use crate::net::network_message::NetworkMessage;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::Player;
use crate::server::world::World;
use tokio::sync::mpsc::UnboundedSender;

// todo: sync

// this is currently a prototype/example idk
// im not sure how i want to handle items
#[derive(Debug)]
pub struct Inventory {
    pub items: [Option<CustomItem>; 36],
}

impl Inventory {
    
    pub fn empty() -> Inventory {
        const NONE: Option<CustomItem> = None;
        Inventory { items: [NONE; 36] }
    }
    
    pub fn set_slot(&mut self, item: CustomItem, slot: usize) {
        self.items[slot] = Some(item);
    }
    
    pub fn get_slot(&self, index: usize) -> &Option<CustomItem> {
        self.items.get(index).unwrap()
    }
}

#[derive(Clone, Debug)]
pub struct CustomItem {
    pub item: Items, 
    pub item_stack: ItemStack,
}

#[derive(Clone, Debug)]
pub enum Items {
    AspectOfTheVoid
}

impl Items {
    pub fn handle_right_click(
        &self, 
        network: &UnboundedSender<NetworkMessage>,
        world: &mut World,
        player: &mut Player,
    ) {
        match self { 
            Items::AspectOfTheVoid => {
                let entity = world.entities.get(&player.entity_id).unwrap();
                // let dist = if player.is_sneaking { 50.0 } else { 20.0 };
                
                let (block_x, block_y, block_z) = {
                    (entity.pos.x.floor() as i32, entity.pos.y.floor() as i32, entity.pos.z.floor() as i32)
                };
                println!(
                    "block {:?} at: x {}, y {}, z {}",
                    world.get_block_at(block_x, block_y, block_z),
                    block_x,
                    block_y,
                    block_z
                );
                
                // player.set_position(
                //     &network,
                //     entity.pos.x,
                //     entity.pos.y + dist,
                //     entity.pos.z,
                //     entity.yaw,
                //     entity.pitch,
                // ).unwrap();
            }
        }
    }
}



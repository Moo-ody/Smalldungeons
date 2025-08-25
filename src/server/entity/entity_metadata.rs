use crate::net::packets::packet_write::PacketWrite;
use crate::server::items::item_stack::ItemStack;

/// Represents an entity type in Minecraft.
#[derive(Debug, Clone)]
pub enum EntityVariant {
    DroppedItem {
        item: ItemStack,
    },
    ArmorStand,
    Zombie {
        is_child: bool,
        is_villager: bool
    },
    Bat {
        hanging: bool
    },
    FallingBlock,
}

impl EntityVariant {

    /// Returns the mc entity id of the variant 
    pub const fn get_id(&self) -> i8 {
        match self {
            EntityVariant::DroppedItem { .. } => 2,
            EntityVariant::ArmorStand => 30,
            EntityVariant::Zombie { .. } => 54,
            EntityVariant::Bat { .. } => 65,
            EntityVariant::FallingBlock => 70,
        }
    }

    /// Returns if the variant is an object and needs to be spawned
    /// using Spawn Object packet instead of Spawn Mob
    pub const fn is_object(&self) -> bool {
        match self {
            EntityVariant::DroppedItem { .. } => true,
            EntityVariant::FallingBlock => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntityMetadata {
    // add more needed stuff here
    pub variant: EntityVariant,
    pub is_invisible: bool
}

impl EntityMetadata {
    pub fn new(variant: EntityVariant) -> Self {
        Self {
            variant,
            is_invisible: false,
        }
    }
}

const BYTE: u8 = 0;
const SHORT: u8 = 1;
const INT: u8 = 2;
const FLOAT: u8 = 3;
const STRING: u8 = 4;
const ITEM_STACK: u8 = 5;

fn write_data(buf: &mut Vec<u8>, data_type: u8, id: u8, data: impl PacketWrite) {
    buf.push((data_type << 5 | id & 31) & 255);
    data.write(buf);
}

impl PacketWrite for EntityMetadata {
    fn write(&self, buf: &mut Vec<u8>) {
        let mut flags: u8 = 0;
        
        if self.is_invisible {
            flags |= 0b00100000 
        }
        
        write_data(buf, BYTE, 0, flags);
        
        match &self.variant {
            EntityVariant::DroppedItem { item } => {
                write_data(buf, ITEM_STACK, 10, Some(item.clone()))
            }
            EntityVariant::Zombie { is_child, is_villager } => {
                write_data(buf, BYTE, 12, *is_child);
                write_data(buf, BYTE, 13, *is_villager);
            }
            EntityVariant::Bat { hanging } => {
                write_data(buf, BYTE, 16, *hanging);
            }
            _ => {}
        }
        buf.push(127)
    }
}
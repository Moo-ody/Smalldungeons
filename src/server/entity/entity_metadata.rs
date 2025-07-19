use crate::net::packets::packet_write::PacketWrite;

/// Represents an entity type in Minecraft.
#[derive(Debug, Clone)]
pub enum EntityVariant {
    DroppedItem,
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
            EntityVariant::DroppedItem => 0,
            EntityVariant::Zombie { .. } => 54,
            EntityVariant::Bat { .. } => 65,
            EntityVariant::FallingBlock => 70,
        }
    }

    /// Returns if the variant is an object and needs to be spawned
    /// using Spawn Object packet instead of Spawn Mob
    pub const fn is_object(&self) -> bool {
        match self {
            EntityVariant::DroppedItem => true,
            EntityVariant::FallingBlock => true,
            _ => false,
        }
    }
}

const BOOL: u8 = 0;
const SHORT: u8 = 1;
const INT: u8 = 2;
const FLOAT: u8 = 3;
const STRING: u8 = 4;
const ITEM_STACK: u8 = 5;

fn write_data(buf: &mut Vec<u8>, data_type: u8, id: u8, data: impl PacketWrite) {
    buf.push((data_type << 5 | id & 31) & 255);
    data.write(buf);
}

impl PacketWrite for EntityVariant {
    fn write(&self, buf: &mut Vec<u8>) {
        match self {
            EntityVariant::Zombie { is_child, is_villager } => {
                write_data(buf, BOOL, 12, *is_child);
                write_data(buf, BOOL, 13, *is_villager);
            }
            EntityVariant::Bat { hanging } => {
                write_data(buf, BOOL, 16, *hanging);
            }
            _ => {}
        }
        buf.push(127)
    }
}
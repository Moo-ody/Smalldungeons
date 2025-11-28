use crate::net::packets::packet_serialize::PacketSerializable;
use crate::server::items::item_stack::ItemStack;
use crate::server::entity::player_skin_bits::SkinParts;

/// Represents an entity type in Minecraft.
#[derive(Debug, Clone)]
pub enum EntityVariant {
    Player,
    DroppedItem {
        item: ItemStack,
    },
    ArmorStand,
    Zombie {
        is_child: bool,
        is_villager: bool,
        is_converting: bool,
        is_attacking: bool,
    },
    Bat {
        hanging: bool
    },
    FallingBlock,
    // NEW: a thrown ender pearl (spawned with Spawn Object)
    EnderPearl,
    // NEW: arrow projectile for Terminator bow
    Arrow,
    // NEW: explosive projectile for Bonzo Staff
    BonzoProjectile,
    // NEW: projectile for Jerry-Chine Gun
    JerryProjectile,
}

impl EntityVariant {

    /// Returns the mc entity id of the variant 
    pub const fn get_id(&self) -> i8 {
        match self {
            // players need to be spawned with SpawnPlayer packet
            EntityVariant::Player => 0, // unused for Player entities
            EntityVariant::DroppedItem { .. } => 2,
            EntityVariant::ArmorStand => 30,
            EntityVariant::Zombie { .. } => 54,
            EntityVariant::Bat { .. } => 65, // mob id (Spawn Mob space)
            EntityVariant::FallingBlock => 70,
            // NEW: object type id for ender pearl (Spawn Object space, 1.8)
            // It's OK that this is also 65 - Spawn Object and Spawn Mob use different id spaces.
            EntityVariant::EnderPearl => 65,
            // NEW: arrow object type id (Spawn Object space, 1.8)
            EntityVariant::Arrow => 60,
            // NEW: bonzo projectile object type id (Spawn Object space, 1.8)
            EntityVariant::BonzoProjectile => 65,
            // NEW: jerry projectile object type id (Spawn Object space, 1.8)
            EntityVariant::JerryProjectile => 65,
        }
    }

    pub const fn is_player(&self) -> bool {
        match self { 
            EntityVariant::Player => true,
            _ => false,
        }
    }
    
    /// Returns if the variant is an object and needs to be spawned
    /// using Spawn Object packet instead of Spawn Mob
    pub const fn is_object(&self) -> bool {
        match self {
            EntityVariant::DroppedItem { .. } => true,
            EntityVariant::FallingBlock => true,
            // NEW
            EntityVariant::EnderPearl => true,
            // NEW: arrows are objects
            EntityVariant::Arrow => true,
            // NEW: bonzo projectiles are objects
            EntityVariant::BonzoProjectile => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntityMetadata {
    // add more needed stuff here
    pub variant: EntityVariant,
    pub is_invisible: bool,
    pub custom_name: Option<String>,
    pub custom_name_visible: bool,
    pub ai_disabled: bool,
    pub skin_parts: Option<SkinParts>, // For Player entities
}

impl EntityMetadata {
    pub fn new(variant: EntityVariant) -> Self {
        let skin_parts = if variant.is_player() {
            Some(SkinParts::default())
        } else {
            None
        };
        
        Self {
            variant,
            is_invisible: false,
            custom_name: None,
            custom_name_visible: false,
            ai_disabled: false,
            skin_parts,
        }
    }
}

const BYTE: u8 = 0;
const SHORT: u8 = 1;
const INT: u8 = 2;
const FLOAT: u8 = 3;
const STRING: u8 = 4;
const ITEM_STACK: u8 = 5;

fn write_data(buf: &mut Vec<u8>, data_type: u8, id: u8, data: impl PacketSerializable) {
    buf.push((data_type << 5 | id & 31) & 255);
    data.write(buf);
}

impl PacketSerializable for EntityMetadata {
    fn write(&self, buf: &mut Vec<u8>) {
        let mut flags: u8 = 0;

        if self.is_invisible {
            flags |= 0b0010_0000; // Bit 5: Invisible
        }

        if self.ai_disabled {
            flags |= 0b0100_0000; // Bit 6: AI disabled
        }

        write_data(buf, BYTE, 0, flags);

        // Add custom name if present
        if let Some(ref name) = self.custom_name {
            write_data(buf, STRING, 2, name.clone());
            // Index 3: CustomNameVisible (BYTE) - required for name to show
            write_data(buf, BYTE, 3, self.custom_name_visible as u8);
        }

        match &self.variant {
            EntityVariant::Player => {
                // Index 10: Player skin parts (0x7E = jacket+sleeves+pants+hat)
                let skin_parts = self.skin_parts.map(|s| s.bits()).unwrap_or(0x7E);
                write_data(buf, BYTE, 10, skin_parts);
            }
            EntityVariant::DroppedItem { item } => {
                // Log the ItemStack right before writing metadata
                eprintln!("[METADATA] Entity metadata ItemStack: id={}, count={}, meta={}, has_nbt={}", 
                    item.item, item.stack_size, item.metadata, item.tag_compound.is_some());
                if item.item == 1 {
                    eprintln!("[METADATA] ERROR: Item ID is 1 (stone) in metadata!");
                }
                write_data(buf, ITEM_STACK, 10, Some(item.clone()))
            }
            EntityVariant::Zombie { is_child, is_villager, is_converting, is_attacking } => {
                write_data(buf, BYTE, 12, *is_child);
                write_data(buf, BYTE, 13, *is_villager);
                write_data(buf, BYTE, 14, *is_converting);
                // Index 15: Try zombie aggressive flag - this controls arm pose in 1.8
                write_data(buf, BYTE, 15, *is_attacking as u8);
            }
            EntityVariant::Bat { hanging } => {
                write_data(buf, BYTE, 16, *hanging);
            }
            // NEW: Ender pearls don't carry extra metadata
            EntityVariant::EnderPearl => { /* no-op */ }
            // NEW: Arrows don't carry extra metadata
            EntityVariant::Arrow => { /* no-op */ }
            // NEW: Bonzo projectiles don't carry extra metadata
            EntityVariant::BonzoProjectile => { /* no-op */ }
            _ => {}
        }
        buf.push(127); // end-of-metadata
    }
}
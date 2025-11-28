use crate::dungeon::room::secrets::{DungeonSecret, SecretType};
use crate::server::block::block_position::BlockPos;
use crate::server::block::rotatable::Rotatable;
use crate::server::items::item_stack::ItemStack;
use crate::server::utils::direction::Direction;
use crate::server::utils::nbt::nbt::{NBT, NBTNode};
use crate::server::utils::nbt::serialize::TAG_COMPOUND_ID;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use once_cell::sync::Lazy;
use rand::Rng;

#[derive(Debug, Deserialize)]
struct SecretJsonData {
    schema: u32,
    rooms: HashMap<String, RoomSecrets>,
}

#[derive(Debug, Deserialize)]
struct RoomSecrets {
    #[serde(default)]
    rchest: Vec<SecretEntry>,
    #[serde(default)]
    ress: Vec<SecretEntry>,
    #[serde(default)]
    batsp: Vec<SecretEntry>,
    #[serde(default)]
    batdie: Vec<SecretEntry>,
    #[serde(default)]
    itemsp: Vec<SecretEntry>,
    #[serde(default)]
    schest: Vec<SecretEntry>,
    #[serde(default)]
    sess: Vec<SecretEntry>,
}


#[derive(Debug, Deserialize, Clone)]
struct SecretEntry {
    id: String,
    x: i32,
    y: i32,
    z: i32,
    #[serde(rename = "facing")]
    #[serde(default)]
    direction: Option<String>,
    #[serde(default)]
    item: Option<ItemJson>,
}

#[derive(Debug, Deserialize, Clone)]
struct ItemJson {
    item: u16,
    #[serde(default)]
    count: u8,
    #[serde(default)]
    damage: u16,
}

static SECRETS_DATA: Lazy<SecretJsonData> = Lazy::new(|| {
    // Load secrets.json from room_data/Chests/secrets.json
    let json_str = include_str!("../../room_data/Chests/secrets.json");
    
    match serde_json::from_str::<SecretJsonData>(json_str) {
        Ok(data) => data,
        Err(_) => {
            SecretJsonData {
                schema: 1,
                rooms: HashMap::new(),
            }
        }
    }
});

pub fn load_secrets_for_room(room_name: &str, corner: BlockPos, rotation: crate::server::utils::direction::Direction) -> Vec<Rc<RefCell<DungeonSecret>>> {
    let mut secrets = Vec::new();
    
    if let Some(room_secrets) = SECRETS_DATA.rooms.get(room_name) {
        let total = room_secrets.rchest.len() + room_secrets.ress.len() + room_secrets.batsp.len() 
            + room_secrets.batdie.len() + room_secrets.itemsp.len() 
            + room_secrets.schest.len() + room_secrets.sess.len();
        // Load secrets for room
        // Process rchest (regular chest)
        for entry in &room_secrets.rchest {
            // Rotate relative position based on room rotation (like crypts)
            let relative_pos = BlockPos {
                x: entry.x,
                y: entry.y, // Y coordinates are absolute (like crypts)
                z: entry.z,
            };
            let rotated = relative_pos.rotate(rotation);
            let world_pos = BlockPos {
                x: corner.x + rotated.x,
                y: rotated.y, // Y coordinates are absolute (like crypts)
                z: corner.z + rotated.z,
            };
            let direction = entry.direction.as_deref()
                .and_then(|d| match d {
                    "north" => Some(Direction::North),
                    "south" => Some(Direction::South),
                    "east" => Some(Direction::East),
                    "west" => Some(Direction::West),
                    _ => None,
                })
                .unwrap_or(Direction::North)
                .rotate(rotation); // Rotate direction based on room rotation
            let secret = DungeonSecret::new(SecretType::RegularChest { direction }, world_pos, 8.0);
            secrets.push(Rc::new(RefCell::new(secret)));
        }
        
        // Process ress (regular essence)
        for entry in &room_secrets.ress {
            // Rotate relative position based on room rotation (like crypts)
            let relative_pos = BlockPos {
                x: entry.x,
                y: entry.y, // Y coordinates are absolute (like crypts)
                z: entry.z,
            };
            let rotated = relative_pos.rotate(rotation);
            let world_pos = BlockPos {
                x: corner.x + rotated.x,
                y: rotated.y, // Y coordinates are absolute (like crypts)
                z: corner.z + rotated.z,
            };
            let secret = DungeonSecret::new(SecretType::RegularEssence, world_pos, 8.0);
            secrets.push(Rc::new(RefCell::new(secret)));
        }
        
        // Process batsp (bat spawn)
        for entry in &room_secrets.batsp {
            // Rotate relative position based on room rotation (like crypts)
            let relative_pos = BlockPos {
                x: entry.x,
                y: entry.y, // Y coordinates are absolute (like crypts)
                z: entry.z,
            };
            let rotated = relative_pos.rotate(rotation);
            let world_pos = BlockPos {
                x: corner.x + rotated.x,
                y: rotated.y, // Y coordinates are absolute (like crypts)
                z: corner.z + rotated.z,
            };
            let secret = DungeonSecret::new(SecretType::BatSpawn { entity_id: None }, world_pos, 8.0);
            secrets.push(Rc::new(RefCell::new(secret)));
        }
        
        // Process batdie (bat die)
        for entry in &room_secrets.batdie {
            // Rotate relative position based on room rotation (like crypts)
            let relative_pos = BlockPos {
                x: entry.x,
                y: entry.y, // Y coordinates are absolute (like crypts)
                z: entry.z,
            };
            let rotated = relative_pos.rotate(rotation);
            let world_pos = BlockPos {
                x: corner.x + rotated.x,
                y: rotated.y, // Y coordinates are absolute (like crypts)
                z: corner.z + rotated.z,
            };
            let secret = DungeonSecret::new(SecretType::BatDie, world_pos, 8.0);
            secrets.push(Rc::new(RefCell::new(secret)));
        }
        
        // Process itemsp (item spawn)
        for entry in &room_secrets.itemsp {
            // Rotate relative position based on room rotation (like crypts)
            let relative_pos = BlockPos {
                x: entry.x,
                y: entry.y, // Y coordinates are absolute (like crypts)
                z: entry.z,
            };
            let rotated = relative_pos.rotate(rotation);
            let world_pos = BlockPos {
                x: corner.x + rotated.x,
                y: rotated.y, // Y coordinates are absolute (like crypts)
                z: corner.z + rotated.z,
            };
            // ItemSpawn secrets don't store the item - it's created fresh at spawn time
            let secret = DungeonSecret::new(SecretType::ItemSpawn, world_pos, 8.0);
            secrets.push(Rc::new(RefCell::new(secret)));
        }
        
        // Process schest (secret chest)
        for entry in &room_secrets.schest {
            // Rotate relative position based on room rotation (like crypts)
            let relative_pos = BlockPos {
                x: entry.x,
                y: entry.y, // Y coordinates are absolute (like crypts)
                z: entry.z,
            };
            let rotated = relative_pos.rotate(rotation);
            let world_pos = BlockPos {
                x: corner.x + rotated.x,
                y: rotated.y, // Y coordinates are absolute (like crypts)
                z: corner.z + rotated.z,
            };
            let direction = entry.direction.as_deref()
                .and_then(|d| match d {
                    "north" => Some(Direction::North),
                    "south" => Some(Direction::South),
                    "east" => Some(Direction::East),
                    "west" => Some(Direction::West),
                    _ => None,
                })
                .unwrap_or(Direction::North)
                .rotate(rotation); // Rotate direction based on room rotation
            let secret = DungeonSecret::new(SecretType::SecretChest { direction }, world_pos, 4.0);
            secrets.push(Rc::new(RefCell::new(secret)));
        }
        
        // Process sess (secret essence)
        for entry in &room_secrets.sess {
            // Rotate relative position based on room rotation (like crypts)
            let relative_pos = BlockPos {
                x: entry.x,
                y: entry.y, // Y coordinates are absolute (like crypts)
                z: entry.z,
            };
            let rotated = relative_pos.rotate(rotation);
            let world_pos = BlockPos {
                x: corner.x + rotated.x,
                y: rotated.y, // Y coordinates are absolute (like crypts)
                z: corner.z + rotated.z,
            };
            let secret = DungeonSecret::new(SecretType::SecretEssence, world_pos, 4.0);
            secrets.push(Rc::new(RefCell::new(secret)));
        }
        
    }
    
    secrets
}

/// Create a Spirit Leap item (enchanted ender pearl)
/// Uses item ID 368 (ender_pearl in 1.8)
pub fn create_spirit_leap_item() -> ItemStack {
    let item = ItemStack {
        item: 368, // ender_pearl
        stack_size: 1,
        metadata: 0,
        tag_compound: Some(NBT::with_nodes(vec![
            NBT::int("HideFlags", 254),
            NBT::compound("display", vec![
                NBT::string("Name", "§aSpirit Leap"),
                NBT::list("Lore", 8, vec![ // 8 = TAG_String
                    NBTNode::String("§7§8Brewing Ingredient".to_string()),
                    NBTNode::String("".to_string()),
                    NBTNode::String("§7Teleports you forward in the".to_string()),
                    NBTNode::String("§7direction you are looking.".to_string()),
                    NBTNode::String("".to_string()),
                    NBTNode::String("§cDungeons only!".to_string()),
                    NBTNode::String("".to_string()),
                    NBTNode::String("§a§lUNCOMMON DUNGEON ITEM".to_string()),
                ]),
            ]),
            NBT::list("ench", TAG_COMPOUND_ID, vec![
                // Add a dummy enchantment to give the glint effect
                NBTNode::Compound({
                    let mut map = std::collections::HashMap::new();
                    map.insert("id".into(), NBTNode::Short(0)); // Protection (any enchant works for glow)
                    map.insert("lvl".into(), NBTNode::Short(1));
                    map
                }),
            ]),
            NBT::compound("ExtraAttributes", vec![
                NBT::string("id", "SPIRIT_LEAP"),
            ]),
        ])),
    };
    
    item
}



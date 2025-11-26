use crate::net::packets::packet_deserialize::PacketDeserializable;
use crate::net::packets::packet_serialize::PacketSerializable;
use crate::server::utils::nbt::deserialize::deserialize_nbt;
use crate::server::utils::nbt::nbt::{NBT, NBTNode};
use crate::server::utils::nbt::serialize::serialize_nbt;
use bytes::{Buf, BytesMut};

#[derive(Debug, Clone, PartialEq)]
pub struct ItemStack {
    pub item: i16,
    pub stack_size: i8,
    pub metadata: i16,
    pub tag_compound: Option<NBT>,
}

impl PacketSerializable for Option<ItemStack> {
    fn write(&self, buf: &mut Vec<u8>) {
        if let Some(item_stack) = self {
            item_stack.item.write(buf);
            item_stack.stack_size.write(buf);
            item_stack.metadata.write(buf);

            match &item_stack.tag_compound {
                None => 0u8.write(buf),
                Some(nbt) => buf.extend(serialize_nbt(nbt)),
            }
        } else {
            (-1i16).write(buf)
        }
    }
}

impl PacketDeserializable for Option<ItemStack> {
    fn read(buffer: &mut BytesMut) -> anyhow::Result<Self> {
        let id = buffer.get_i16();
        if id >= 0 {
            let item_stack = ItemStack {
                item: id,
                stack_size: buffer.get_i8(),
                metadata: buffer.get_i16(),
                tag_compound: deserialize_nbt(buffer),
            };
            return Ok(Some(item_stack));
        }
        Ok(None)
    }
}

impl ItemStack {
    /// Create a new ItemStack with the given item ID
    pub fn new(item: i16) -> Self {
        Self {
            item,
            stack_size: 1,
            metadata: 0,
            tag_compound: None,
        }
    }

    /// Set the unbreakable flag on this ItemStack
    pub fn set_unbreakable(&mut self, unbreakable: bool) {
        if unbreakable {
            if self.tag_compound.is_none() {
                self.tag_compound = Some(NBT::with_nodes(vec![]));
            }
            if let Some(ref mut tag) = self.tag_compound {
                tag.nodes.insert("Unbreakable".into(), NBTNode::Byte(1));
                tag.nodes.insert("HideFlags".into(), NBTNode::Int(127));
            }
        }
    }
}

/// Enchantment types for equipment
#[derive(Debug, Clone, Copy)]
pub enum Enchant {
    Sharpness,
    Protection,
    // Add more as needed
}

impl Enchant {
    pub const fn get_id(&self) -> i16 {
        match self {
            Enchant::Sharpness => 16,  // Sharpness enchant ID in 1.8
            Enchant::Protection => 0,  // Protection enchant ID in 1.8
        }
    }
}

/// Extension trait for ItemStack with builder-like methods
pub trait ItemStackExt {
    fn unbreakable(self) -> Self;
    fn ench(self, kind: Enchant, lvl: i32) -> Self;
    fn leather_rgb(self, r: u8, g: u8, b: u8) -> Self;
}

impl ItemStackExt for ItemStack {
    fn unbreakable(mut self) -> Self {
        self.set_unbreakable(true);
        self
    }

    fn ench(mut self, kind: Enchant, lvl: i32) -> Self {
        if self.tag_compound.is_none() {
            self.tag_compound = Some(NBT::with_nodes(vec![]));
        }
        
        if let Some(ref mut tag) = self.tag_compound {
            // Get or create enchantments list
            let enchant_id = kind.get_id();
            let enchant_node = NBTNode::Compound({
                let mut map = std::collections::HashMap::new();
                map.insert("id".into(), NBTNode::Short(enchant_id));
                map.insert("lvl".into(), NBTNode::Short(lvl as i16));
                map
            });

            if let Some(NBTNode::List { children, .. }) = tag.nodes.get_mut("ench") {
                children.push(enchant_node);
            } else {
                // Create new enchantments list
                tag.nodes.insert("ench".into(), NBTNode::List {
                    type_id: 10, // TAG_Compound
                    children: vec![enchant_node],
                });
            }
        }
        self
    }

    fn leather_rgb(mut self, r: u8, g: u8, b: u8) -> Self {
        // Only apply to leather armor items (helmets, chestplates, leggings, boots with specific IDs)
        let is_leather = matches!(self.item, 298 | 299 | 300 | 301); // leather armor item IDs
        
        if is_leather && self.tag_compound.is_none() {
            self.tag_compound = Some(NBT::with_nodes(vec![]));
        }
        
        if is_leather {
            if let Some(ref mut tag) = self.tag_compound {
                // Create color from RGB values (RGB packed into an integer)
                let color = ((r as i32) << 16) | ((g as i32) << 8) | (b as i32);
                tag.nodes.insert("display".into(), NBTNode::Compound({
                    let mut display_map = std::collections::HashMap::new();
                    display_map.insert("color".into(), NBTNode::Int(color));
                    display_map
                }));
            }
        }
        self
    }
}
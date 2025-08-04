use crate::server::utils::nbt::serialize::TAG_STRING_ID;

pub mod serialize;
pub mod deserialize;

/// NBT
/// 
/// This struct represents the root NBT Tag Compound.
#[derive(Debug, Clone, PartialEq)]
pub struct NBT {
    pub root_name: String,
    pub nodes: Vec<(String, NBTNode)>,
}

impl NBT {
    
    /// Creates a new [NBT] struct with the provided nodes.
    /// It does not have a root name
    pub fn with_nodes(nodes: Vec<(String, NBTNode)>) -> NBT {
        NBT {
            root_name: "".to_string(),
            nodes,
        }
    }
    
    /// creates a string nbt node
    /// used for [NBTNode::Compound] and [NBT]
    pub fn string(name: &str, value: &str) -> (String, NBTNode) {
        (name.to_string(), NBTNode::String(value.to_string()))
    }

    /// creates a compound node
    /// used for [NBTNode::Compound] and [NBT]
    pub fn compound(name: &str, nodes: Vec<(String, NBTNode)>) -> (String, NBTNode) {
        (name.to_string(), NBTNode::Compound(nodes))
    }

    /// creates a list node
    /// used for [NBTNode::Compound] and [NBT]
    pub fn list(name: &str, type_id: u8, list: Vec<NBTNode>) -> (String, NBTNode) {
        (name.to_string(), NBTNode::List { type_id, children: list })
    }

    pub fn byte(name: &str, value: i8) -> (String, NBTNode) {
        (name.to_string(), NBTNode::Byte(value))
    }

    pub fn short(name: &str, value: i16) -> (String, NBTNode) {
        (name.to_string(), NBTNode::Short(value))
    }

    pub fn int(name: &str, value: i32) -> (String, NBTNode) {
        (name.to_string(), NBTNode::Int(value))
    }

    pub fn long(name: &str, value: i64) -> (String, NBTNode) {
        (name.to_string(), NBTNode::Long(value))
    }
    
    /// takes a string,
    /// splits it into lines and creates a list nbt node representing strings.
    pub fn list_from_string(name: &str, string: &str) -> (String, NBTNode) {
        let list = string
            .lines()
            .map(|line| NBTNode::String(line.to_string()))
            .collect();
        (name.to_string(), NBTNode::List { type_id: TAG_STRING_ID, children: list })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NBTNode {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List { type_id: u8, children: Vec<NBTNode> },
    Compound(Vec<(String, NBTNode)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}
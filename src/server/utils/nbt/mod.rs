pub mod encode;
mod decode;

/// NBT
/// 
/// This struct represents the root NBT Tag Compound.
#[derive(Debug, Clone)]
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
    
    /// creates a string nbt node used for [NBTNode::Compound] and [NBT]
    pub fn string(name: &str, value: &str) -> (String, NBTNode) {
        (name.to_string(), NBTNode::String(value.to_string()))
    }

    /// creates a compound node used for [NBTNode::Compound] and [NBT]
    pub fn compound(names: &str, nodes: Vec<(String, NBTNode)>) -> (String, NBTNode) {
        (names.to_string(), NBTNode::Compound(nodes))
    }
}

#[derive(Debug, Clone)]
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
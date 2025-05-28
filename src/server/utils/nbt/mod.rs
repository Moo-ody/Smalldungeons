#[derive(Debug)]
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

pub const TAG_END_ID: u8 = 0;
pub const TAG_BYTE_ID: u8 = 1;
pub const TAG_SHORT_ID: u8 = 2;
pub const TAG_INT_ID: u8 = 3;
pub const TAG_LONG_ID: u8 = 4;
pub const TAG_FLOAT_ID: u8 = 5;
pub const TAG_DOUBLE_ID: u8 = 6;
pub const TAG_BYTE_ARRAY_ID: u8 = 7;
pub const TAG_STRING_ID: u8 = 8;
pub const TAG_LIST_ID: u8 = 9;
pub const TAG_COMPOUND_ID: u8 = 10;
pub const TAG_INT_ARRAY_ID: u8 = 11;
pub const TAG_LONG_ARRAY_ID: u8 = 12;

fn get_tag(node: &NBTNode) -> u8 {
    match node {
        NBTNode::Byte(_) => TAG_BYTE_ID,
        NBTNode::Short(_) => TAG_SHORT_ID,
        NBTNode::Int(_) => TAG_INT_ID,
        NBTNode::Long(_) => TAG_LONG_ID,
        NBTNode::Float(_) => TAG_FLOAT_ID,
        NBTNode::Double(_) => TAG_DOUBLE_ID,
        NBTNode::ByteArray(_) => TAG_BYTE_ARRAY_ID,
        NBTNode::String(_) => TAG_STRING_ID,
        NBTNode::List { .. } => TAG_LIST_ID,
        NBTNode::Compound(_) => TAG_COMPOUND_ID,
        NBTNode::IntArray(_) => TAG_INT_ARRAY_ID,
        NBTNode::LongArray(_) => TAG_LONG_ARRAY_ID,
    }
}

// ill assume compound is there by default
pub fn serialize(nbt: &NBTNode) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();
    data.push(TAG_COMPOUND_ID);
    data.push(0);
    data.push(0);
    serialize_to_payload(&mut data, nbt);
    // data.push(TAG_END_ID);
    data
}

pub fn serialize_to_payload(payload: &mut Vec<u8>, nbt: &NBTNode) {
    match nbt {
        NBTNode::Byte(value) => payload.extend_from_slice(&value.to_be_bytes()),
        NBTNode::Short(value) => payload.extend_from_slice(&value.to_be_bytes()),
        NBTNode::Int(value) => payload.extend_from_slice(&value.to_be_bytes()),
        NBTNode::Long(value) => payload.extend_from_slice(&value.to_be_bytes()),
        NBTNode::Float(value) => payload.extend_from_slice(&value.to_be_bytes()),
        NBTNode::Double(value) => payload.extend_from_slice(&value.to_be_bytes()),

        NBTNode::ByteArray(value) => {
            payload.extend_from_slice(&(value.len() as i32).to_be_bytes());
            payload.extend_from_slice(&value);
        }
        NBTNode::String(value) => {
            let str_bytes = value.as_bytes();
            payload.extend_from_slice(&(str_bytes.len() as u16).to_be_bytes());
            payload.extend_from_slice(str_bytes);
        }
        // do later, dont worry
        NBTNode::List { type_id, children } => {},
        NBTNode::Compound(map) => {
            for (name, node) in map {
                payload.push(get_tag(node));
                let str_bytes = name.as_bytes();
                payload.extend_from_slice(&(str_bytes.len() as u16).to_be_bytes());
                payload.extend_from_slice(str_bytes);
                serialize_to_payload(payload, node);
            }
            payload.push(TAG_END_ID);
        }
        NBTNode::IntArray(value) => {
            payload.extend_from_slice(&(value.len() as i32).to_be_bytes());
            for val in value {
                payload.extend_from_slice(&val.to_be_bytes());
            }
        }
        NBTNode::LongArray(value) => {
            payload.extend_from_slice(&(value.len() as i32).to_be_bytes());
            for val in value {
                payload.extend_from_slice(&val.to_be_bytes());
            }
        }
    }
}
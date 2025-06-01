use crate::server::utils::nbt::NBT;
use bytes::{Buf, BytesMut};


// todo : finish
pub fn deserialize_nbt(bytes: &mut BytesMut) -> Option<NBT> {
    if bytes.remaining() < 1 {
        return None
    }
    let tag = bytes.chunk()[0];
    if tag == 0 {
        return None
    } else {
        bytes.advance(1);
    }
    None
}
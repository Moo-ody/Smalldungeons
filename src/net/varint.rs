use bytes::{Buf, BytesMut};
use crate::net::packets::packet::PacketWrite;

pub struct VarInt(pub i32);
impl PacketWrite for VarInt {
    fn write(&self, buf: &mut Vec<u8>) {
        write_varint(buf, self.0);
    }
}

pub fn read_varint(buf: &mut BytesMut) -> Option<i32> {
    let mut num_read = 0;
    let mut result = 0i32;

    loop {
        if num_read >= 5 || num_read >= buf.len() {
            return None;
        }

        let byte = buf[num_read];
        let value = (byte & 0x7f) as i32;
        result |= value << (7 * num_read);
        num_read += 1;

        if byte & 0x80 == 0 {
            break;
        }
    }

    buf.advance(num_read);
    Some(result)
}

pub fn write_varint(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        if (value & !0x7F) == 0 {
            buf.push(value as u8);
            return;
        } else {
            buf.push(((value & 0x7F) | 0x80) as u8);
            value >>= 7;
        }
    }
}
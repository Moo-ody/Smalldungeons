use crate::net::packets::packet_write::PacketWrite;
use bytes::{Buf, BytesMut};

pub struct VarInt(pub i32);

impl PacketWrite for VarInt {
    fn write(&self, buf: &mut Vec<u8>) {
        write_var_int(buf, self.0);
    }
}

/// this does NOT advance the buffer, strictly reads from it.
pub fn read_var_int_with_len(buf: &BytesMut) -> Option<(i32, usize)> {
    let mut num_read = 0;
    let mut result = 0;

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

    Some((result, num_read))
}

pub fn read_var_int(buf: &mut BytesMut) -> Option<i32> {
    let (int, len) = read_var_int_with_len(buf)?;
    buf.advance(len);
    Some(int)
}

pub fn write_var_int(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        if (value & !0x7F) == 0 {
            buf.push(value as u8);
            return;
        }
        buf.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
}
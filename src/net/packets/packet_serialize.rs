use crate::net::var_int::{write_var_int, VarInt};
use uuid::Uuid;

pub trait PacketSerializable {
    fn write(&self, buf: &mut Vec<u8>);
}

impl PacketSerializable for VarInt {
    fn write(&self, buf: &mut Vec<u8>) {
        write_var_int(buf, self.0)
    }
}

impl PacketSerializable for bool {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.push(*self as u8)
    }
}

impl PacketSerializable for u8 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.push(*self);
    }
}

impl PacketSerializable for i8 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.push(*self as u8);
    }
}

impl PacketSerializable for u16 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketSerializable for i16 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketSerializable for u32 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketSerializable for i32 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes())
    }
}

impl PacketSerializable for i64 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketSerializable for f32 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketSerializable for f64 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketSerializable for &[u8] {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self);
    }
}

impl<const N: usize> PacketSerializable for &[u8; N] {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self[..]);
    }
}

impl PacketSerializable for &str {
    fn write(&self, buf: &mut Vec<u8>) {
        write_var_int(buf, self.len() as i32);
        buf.extend_from_slice(self.as_bytes());
    }
}

impl PacketSerializable for String {
    fn write(&self, buf: &mut Vec<u8>) {
        write_var_int(buf, self.len() as i32);
        buf.extend_from_slice(self.as_bytes());
    }
}

// I don't know if this is a good idea,
// maybe have a wrapper type that writes the length
impl<T: PacketSerializable> PacketSerializable for Vec<T> {
    fn write(&self, buf: &mut Vec<u8>) {
        write_var_int(buf, self.len() as i32);
        for entry in self {
            entry.write(buf)
        }
    }
}

impl PacketSerializable for Uuid {
    fn write(&self, buf: &mut Vec<u8>) {
        let bytes = self.as_u128();
        let most = (bytes >> 64) as i64;
        let least = bytes as i64;
        most.write(buf);
        least.write(buf);
    }
}
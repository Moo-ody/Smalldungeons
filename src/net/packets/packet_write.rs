use crate::net::varint::write_varint;
use uuid::Uuid;

pub trait PacketWrite {
    fn write(&self, buf: &mut Vec<u8>);
}

impl PacketWrite for bool {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.push(*self as u8)
    }
}

impl PacketWrite for u8 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.push(*self);
    }
}

impl PacketWrite for i8 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.push(*self as u8);
    }
}

impl PacketWrite for u16 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for i16 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for u32 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for i32 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for i64 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for f32 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for f64 {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self.to_be_bytes());
    }
}

impl PacketWrite for &[u8] {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self);
    }
}

impl<const N: usize> PacketWrite for &[u8; N] {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(&self[..]);
    }
}

impl PacketWrite for &str {
    fn write(&self, buf: &mut Vec<u8>) {
        write_varint(buf, self.len() as i32);
        buf.extend_from_slice(self.as_bytes());
    }
}

impl PacketWrite for String {
    fn write(&self, buf: &mut Vec<u8>) {
        write_varint(buf, self.len() as i32);
        buf.extend_from_slice(self.as_bytes());
    }
}

impl PacketWrite for Uuid {
    fn write(&self, buf: &mut Vec<u8>) {
        let bytes = self.as_u128();
        let most = (bytes >> 64) as i64;
        let least = bytes as i64;

        most.write(buf);
        least.write(buf);
    }
}
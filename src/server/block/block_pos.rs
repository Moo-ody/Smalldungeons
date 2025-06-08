use crate::{net::packets::packet_write::PacketWrite, server::utils::direction::Direction};
use bytes::{Buf, BytesMut};

#[derive(Debug, Clone)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub fn is_invalid(&self) -> bool {
        self.x.is_negative() || self.y.is_negative() || self.z.is_negative()
    }

    pub fn rotate(&self, rotation: Direction) -> BlockPos {
        match rotation {
            Direction::North => BlockPos { x: self.x, y: self.y, z: self.z },
            Direction::East => BlockPos { x: self.z, y: self.y, z: -self.x },
            Direction::South => BlockPos { x: -self.x, y: self.y, z: -self.z },
            Direction::West => BlockPos { x: -self.z, y: self.y, z: self.x },
        }
    }
}

impl PacketWrite for BlockPos {
    fn write(&self, buf: &mut Vec<u8>) {
        let long: i64 = (self.x as i64 & XZ_MASK) << X_SHIFT | (self.y as i64 & Y_MASK) << Y_SHIFT | (self.z as i64 & XZ_MASK);
        long.write(buf);
    }
}

pub fn read_block_pos(buf: &mut BytesMut) -> BlockPos {
    let long = buf.get_i64();
    BlockPos {
        x: (long << (64 - X_SHIFT - XZ_BITS) >> (64 - XZ_BITS)) as i32,
        y: (long << (64 - Y_SHIFT - Y_BITS) >> (64 - Y_BITS)) as i32,
        z: (long << (64 - XZ_BITS) >> (64 - XZ_BITS)) as i32,
    }
}

const XZ_BITS: i32 = 26;
const Y_BITS: i32 = 12;


const X_SHIFT: i32 = 38;
const Y_SHIFT: i32 = 26;

const XZ_MASK: i64 = 0x3FFFFFF;
const Y_MASK: i64 = 0xFFF;


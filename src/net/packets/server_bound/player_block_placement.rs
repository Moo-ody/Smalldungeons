use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::server::items::item_stack::ItemStack;
use crate::server::world::World;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct PlayerBlockPlacement {
    pub position: u64,
    pub placed_direction: u8,
    pub item_stack: Option<ItemStack>,
    // pub item_stack: i16, // currently only accepting null itemstacks
    pub facing_x: f32,
    pub facing_y: f32,
    pub facing_z: f32,
}

#[async_trait::async_trait]
impl ServerBoundPacket for PlayerBlockPlacement {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        let packet = PlayerBlockPlacement {
            position: buf.get_u64(),
            placed_direction: buf.get_u8(),
            item_stack: get_item_stack(buf),
            facing_x: buf.get_u8() as f32 / 16.0,
            facing_y: buf.get_u8() as f32 / 16.0,
            facing_z: buf.get_u8() as f32 / 16.0,
        };
        Ok(packet)
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        Ok(())
    }
}

fn get_item_stack(buf: &mut BytesMut) -> Option<ItemStack> {
    let id = buf.get_i16();
    if id >= 0 {
        let item_stack = ItemStack {
            item: id,
            stack_size: buf.get_i8(),
            metadata: buf.get_i16(),
            tag_compound: None,
        };
        return Some(item_stack);
    }
    None
}

// fn read_nbt_tag_compound(buf: &mut BytesMut) -> Option<NBTTagCompound> {
//     if buf.is_empty() {
//         return None;
//     }
//
//     // Peek the first byte
//     let first_byte = buf[0];
//     if first_byte == 0 {
//         buf.advance(1);
//         return None;
//     }
//
//     // Decompress the buffer using GZIP
//     let cursor = Cursor::new(buf.as_ref());
//     let mut decoder = ZlibDecoder::new(cursor);
//
//     let mut decompressed = Vec::new();
//     if decoder.read_to_end(&mut decompressed).is_err() {
//         return None;
//     }
//
//     // Create an input slice for the NBT reader
//     let mut input: &[u8] = &decompressed;
//
//     let mut compound = NBTTagCompound::new();
//     let mut tracker = NBTSizeTracker::new(2_097_152); // same as Java
//
//     if compound.read(&mut input, 0, &mut tracker).is_ok() {
//         println!("hello");
//         Some(compound)
//     } else {
//         None
//     }
// }
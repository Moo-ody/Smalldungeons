use crate::net::packets::packet::ServerBoundPacket;
use crate::server::items::item_stack::ItemStack;
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
            // todo: unpack
            position: buf.get_u64(),
            placed_direction: buf.get_u8(),
            item_stack: read_item_stack(buf),
            facing_x: buf.get_u8() as f32 / 16.0,
            facing_y: buf.get_u8() as f32 / 16.0,
            facing_z: buf.get_u8() as f32 / 16.0,
        };
        Ok(packet)
    }
}

// todo, have this in its own file
fn read_item_stack(buf: &mut BytesMut) -> Option<ItemStack> {
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
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::server::items::item_stack::ItemStack;
use crate::server::old_world::World;
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
            item_stack: read_item_stack(buf),
            facing_x: buf.get_u8() as f32 / 16.0,
            facing_y: buf.get_u8() as f32 / 16.0,
            facing_z: buf.get_u8() as f32 / 16.0,
        };
        println!("!!! item stack = {:?}", &packet.item_stack);
        Ok(packet)
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        Ok(())
    }
}

// todo, have this in its own file
fn read_item_stack(buf: &mut BytesMut) -> Option<ItemStack> {
    let id = buf.get_i16();
    if id >= 0 {
        let item_stack = ItemStack {
            item: id,
            stack_size: buf.get_u8(),
            metadata: buf.get_u16(),
            tag_compound: None,
        };
        return Some(item_stack);
    }
    None
}
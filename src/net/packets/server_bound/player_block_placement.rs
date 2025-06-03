use crate::net::packets::client_bound::block_change::BlockChange;
use crate::net::packets::packet::{SendPacket, ServerBoundPacket};
use crate::server::block::block_pos::{read_block_pos, BlockPos};
use crate::server::items::item_stack::ItemStack;
use crate::server::player::Player;
use crate::server::world::World;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct PlayerBlockPlacement {
    pub block_pos: BlockPos,
    pub placed_direction: u8,
    pub item_stack: Option<ItemStack>,
    pub facing_x: f32,
    pub facing_y: f32,
    pub facing_z: f32,
}

#[async_trait::async_trait]
impl ServerBoundPacket for PlayerBlockPlacement {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        let packet = PlayerBlockPlacement {
            block_pos: read_block_pos(buf),
            placed_direction: buf.get_u8(),
            item_stack: read_item_stack(buf),
            facing_x: buf.get_u8() as f32 / 16.0,
            facing_y: buf.get_u8() as f32 / 16.0,
            facing_z: buf.get_u8() as f32 / 16.0,
        };
        // println!("packet: {:?}", packet);
        Ok(packet)
    }

    fn main_process(&self, world: &mut World, player: &mut Player) -> anyhow::Result<()> {
        if self.item_stack.is_some() {
            if !self.block_pos.is_invalid() {
                let mut bp = self.block_pos.clone();
                match self.placed_direction {
                    0 => bp.y -= 1,
                    1 => bp.y += 1,
                    2 => bp.z -= 1,
                    3 => bp.z += 1,
                    4 => bp.x -= 1,
                    _ => bp.x += 1
                }
                let block = world.get_block_at(bp.x, bp.y, bp.z);
                BlockChange {
                    block_pos: bp,
                    block_state: block.block_state_id(),
                }.send_packet(player.client_id, &player.server_mut().network_tx)?;
            }
            player.handle_right_click()
        }
        Ok(())
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
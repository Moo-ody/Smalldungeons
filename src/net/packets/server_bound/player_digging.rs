use crate::net::packets::client_bound::block_change::BlockChange;
use crate::net::packets::packet::{SendPacket, ServerBoundPacket};
use crate::net::var_int::read_var_int;
use crate::server::block::block_pos::{read_block_pos, BlockPos};
use crate::server::block::blocks::Blocks;
use crate::server::items::Item;
use crate::server::player::inventory::ItemSlot;
use crate::server::player::Player;
use crate::server::utils::direction::Direction;
use crate::server::world::World;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct PlayerDigging {
    action: PlayerDiggingAction,
    position: BlockPos,
    facing: Direction
}

#[derive(Debug)]
enum PlayerDiggingAction {
    StartDestroyBlock,
    AbortDestroyBlock,
    FinishDestoryBlock,
    DropAllItem, // <-- don't really need these
    DropItem, //    <--
    ReleaseUseItem, // bow nd shi
}

#[async_trait::async_trait]
impl ServerBoundPacket for PlayerDigging {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        let e = Self {
            action: match read_var_int(buf).unwrap() {
                0 => PlayerDiggingAction::StartDestroyBlock,
                1 => PlayerDiggingAction::AbortDestroyBlock,
                2 => PlayerDiggingAction::FinishDestoryBlock,
                3 => PlayerDiggingAction::DropAllItem,
                4 => PlayerDiggingAction::DropItem,
                _ => PlayerDiggingAction::ReleaseUseItem,
            },
            position: read_block_pos(buf),
            facing: match buf.get_u8() % 6 {
                0 => Direction::Down,
                1 => Direction::Up,
                2 => Direction::North,
                3 => Direction::South,
                4 => Direction::West,
                _ => Direction::East,
            },
        };
        Ok(e)
    }

    fn main_process(&self, world: &mut World, player: &mut Player) -> anyhow::Result<()> {
        match self.action {
            PlayerDiggingAction::StartDestroyBlock => {
                if let Some(ItemSlot::Filled(Item::DiamondPickaxe)) = player.inventory.get_hotbar_slot(player.held_slot as usize) {
                    let bp = self.position.clone();
                    let block = world.get_block_at(bp.x, bp.y, bp.z);
                    if let Blocks::Skull { .. } = block {
                        println!("block {:?}", block);
                    }
                    BlockChange {
                        block_pos: bp,
                        block_state: block.get_blockstate_id(),
                    }.send_packet(player.client_id, &player.server_mut().network_tx)?;
                }
            }
            PlayerDiggingAction::FinishDestoryBlock => {
                let bp = self.position.clone();
                let block = world.get_block_at(bp.x, bp.y, bp.z);
                BlockChange {
                    block_pos: bp,
                    block_state: block.get_blockstate_id(),
                }.send_packet(player.client_id, &player.server_mut().network_tx)?;
            }
            _ => {}
        }
        Ok(())
    }
}
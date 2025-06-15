use crate::net::packets::packet::ServerBoundPacket;
use crate::server::player::Player;
use crate::server::world::World;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct HeldItemChange {
    // for some reason this is a short
    pub slot_id: u16
}

#[async_trait::async_trait]
impl ServerBoundPacket for HeldItemChange {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(HeldItemChange {
            slot_id: buf.get_u16(),
        })
    }

    fn main_process(&self, _: &mut World, player: &mut Player) -> anyhow::Result<()> {
        // im not sure if this can overflow, but to be safe
        let item_slot = self.slot_id.clamp(0, 8) as u8;
        player.held_slot = item_slot;
        Ok(())
    }
}
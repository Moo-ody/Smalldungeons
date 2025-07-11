use crate::net::packets::packet::ServerBoundPacket;
use crate::server::items::item_stack::{read_item_stack, ItemStack};
use crate::server::player::Player;
use crate::server::world::World;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct ClickWindow {
    pub window_id: i8,
    pub slot_id: i16, // normal inven slot or -999
    pub used_button: i8, // 0..=10
    pub action_number: i16,
    pub clicked_item: Option<ItemStack>,
    pub mode: ClickMode,
    // pub mode: i8 // 0..=6,
}

#[repr(u8)]
#[derive(Debug)]
pub enum ClickMode {
    NormalClick,
    ShiftClick,
    NumberKey,
    MiddleClick,
    Drop,
    Drag, // this one i cba implementing fully
    DoubleClick // this one i cba implementing fully,
}

#[async_trait::async_trait]
impl ServerBoundPacket for ClickWindow {

    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok({
            let window_id = buf.get_i8();
            if window_id.is_negative() { /* maybe warn idk */ }
            let slot_id = buf.get_i16();
            let used_button = buf.get_i8();
            let action_number = buf.get_i16();
            let mode = match buf.get_i8() { 
                0 => ClickMode::NormalClick,
                1 => ClickMode::ShiftClick,
                2 => ClickMode::NumberKey,
                3 => ClickMode::MiddleClick,
                4 => ClickMode::Drop,
                5 => ClickMode::Drag,
                _ => ClickMode::DoubleClick,
            };
            let clicked_item = read_item_stack(buf);
            ClickWindow { window_id, slot_id, used_button, action_number, clicked_item, mode }
        })
    }

    fn main_process(&self, _: &mut World, player: &mut Player) -> anyhow::Result<()> {
        player.current_ui.clone().handle_click_window(self, player)?;
        // player.inventory.handle_click_window(self, &player.client_id, &network_tx)?;

        println!("click window packet {:?}", self);
        // player.inventory.sync(player, &player.server_mut().network_tx)?;
        Ok(())
    }
}
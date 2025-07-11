use crate::net::packets::packet::ServerBoundPacket;
use crate::server::player::ui::UI;
use crate::server::player::Player;
use crate::server::world::World;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct CloseWindowPacket {
    pub window_id: i8,
}

#[async_trait::async_trait]
impl ServerBoundPacket for CloseWindowPacket {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(CloseWindowPacket {
            window_id: buf.get_i8(),
        })
    }

    fn main_process(&self, _: &mut World, player: &mut Player) -> anyhow::Result<()> {
        // todo in wd branch, implement transaction packet syncing with this 
        player.open_ui(UI::None)?;
        Ok(())
    }
}
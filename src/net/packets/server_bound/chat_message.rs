use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::packets::read_string_from_buf;
use crate::server::player::Player;
use crate::server::world::World;
use bytes::BytesMut;

#[derive(Debug)]
pub struct ChatMessage {
    message: String,
}

#[async_trait::async_trait]
impl ServerBoundPacket for ChatMessage {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(Self {
            message: read_string_from_buf(buf, 100)? // vanilla has a limit of 100 but i dont think thats strictly necessary.
        })
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        // idrk if this is handled on tick or not but
        // if self.message.starts_with("/") {
        //     if self.message == "/updatezombie" {
        //
        //     }
        // } else {
        //     // forward to 1v1 player?
        // }
        Ok(())
    }

    fn main_process(&self, world: &mut World, player: &mut Player) -> anyhow::Result<()> {
        // todo
        Ok(())
    }
}
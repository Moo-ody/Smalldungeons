use crate::net::packets::old_packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::packets::read_string_from_buf;
use crate::server::player::player::Player;
use crate::server::world::World;
use bytes::BytesMut;

#[derive(Debug)]
pub struct ChatMessage {
    pub message: String,
}

#[async_trait::async_trait]
impl ServerBoundPacket for ChatMessage {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(Self {
            message: read_string_from_buf(buf, 100)? // vanilla has a limit of 100 but i dont think thats strictly necessary.
        })
    }

    async fn process<'a>(&self, context: PacketContext<'a>) -> anyhow::Result<()> {
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
        if self.message == "/locraw" {
            player.send_message(r#"{"server":"mini237V","gametype":"SKYBLOCK","mode":"dungeon","map":"Dungeon"}"#);
        }/* else if self.message == "/mort" {
            player.open_ui(UI::MortReadyUpMenu)?;
        };*/

        // if self.message.starts_with("/") && self.message == "/updatezombie" {
        //     let mut id: Option<EntityId> = None;
        //     let mut path: Option<Vec<BlockPos>> = None;
        //     if let Some((entity_id, e)) = world.entities.iter().find(|(_, e)| e.metadata.base.name == "Zombie") {
        //         path = Pathfinder::find_path(e, &BlockPos { x: 10, y: 1, z: 10 }, world).ok();
        //         id = Some(*entity_id)
        //     }
        //     
        //     if let Some((id, path)) = id.and_then(|id| path.map(|path| (id, path))) {
        //         if let Some(e) = world.entities.get_mut(&id) { 
        //             e.path = Some(path)
        //         }
        //     }
        //     
        // }
        // todo
        Ok(())
    }
}
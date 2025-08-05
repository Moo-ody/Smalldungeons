// use crate::net::packets::packet::ServerBoundPacket;
// use crate::net::packets::read_string_from_buf;
// use crate::server::player::player::Player;
// use crate::server::utils::commands::Command;
// use crate::server::world::World;
// use anyhow::Context;
// use bytes::BytesMut;
//
// #[derive(Debug)]
// pub struct ChatMessage {
//     pub message: String,
// }
//
// #[async_trait::async_trait]
// impl ServerBoundPacket for ChatMessage {
//     async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
//         Ok(Self {
//             message: read_string_from_buf(buf, 100)? // vanilla has a limit of 100 but i dont think thats strictly necessary.
//         })
//     }
//
//     fn main_process(&self, world: &mut World, player: &mut Player) -> anyhow::Result<()> {
//         if self.message.starts_with("/") {
//             let command = self.message.strip_prefix("/").context("Failed to strip prefix from command")?;
//             Command::handle(command, world, player)?;
//         }
//
//         // todo normal chat messages maybe?
//         Ok(())
//     }
// }
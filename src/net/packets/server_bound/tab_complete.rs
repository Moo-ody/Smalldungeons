use crate::net::packets::client_bound::tab_complete::TabComplete as CBTabComplete;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::read_string_from_buf;
use crate::server::block::block_pos::{read_block_pos, BlockPos};
use crate::server::player::player::Player;
use crate::server::utils::commands::Command;
use crate::server::world::World;
use anyhow::Context;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct TabComplete {
    message: String,
    target_block: Option<BlockPos>,
}

#[async_trait::async_trait]
impl ServerBoundPacket for TabComplete {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(Self {
            message: read_string_from_buf(buf, 32767)?,
            target_block: if buf.get_u8() != 0 {
                Some(read_block_pos(buf))
            } else {
                None
            },
        })
    }

    fn main_process(&self, world: &mut World, player: &mut Player) -> anyhow::Result<()> {
        if !self.message.starts_with("/") {
            // if we do non command based tab completion, well need to do it here or so.
            return Ok(());
        }

        let parts: Vec<&str> = self.message.split_whitespace().collect();
        let command_name = parts[0].strip_prefix("/").context("Failed to strip prefix from command")?;

        if command_name.is_empty() {
            player.send_packet(CBTabComplete {
                matches: Command::list().iter().map(|cmd| format!("/{}", cmd.name())).collect()
            })?;
            return Ok(());
        }

        if let Some(command) = Command::find(command_name) {
            let args = &parts[1..];

            let next_arg = self.message.ends_with(' ');

            if args.is_empty() && !next_arg {
                // user input a valid command but has not hit space, so we shouldn't provide any completions.
                // there might be a better way to do this somewhere else but idk atm.
                return Ok(());
            }

            let current_arg = if next_arg {
                args.len()
            } else {
                args.len().saturating_sub(1)
            };

            let command_args = command.args(world, player);

            if current_arg >= command_args.len() {
                // user has input too many arguments; so we just return here.
                return Ok(());
            }

            let completions = &command_args.get(current_arg).context("Failed to get completions for command")?.completions;

            let matches: Vec<String> = if next_arg || args.is_empty() {
                completions.to_vec()
            } else {
                completions.iter().filter(|cmp| cmp.starts_with(args.last().unwrap_or(&""))).cloned().collect()
            };

            player.send_packet(CBTabComplete {
                matches
            })?;
        } else {
            let commands = Command::list().iter().filter(|cmd| cmd.name().starts_with(command_name)).map(|cmd| format!("/{}", cmd.name())).collect();

            player.send_packet(CBTabComplete {
                matches: commands
            })?;
        }

        Ok(())
    }
}
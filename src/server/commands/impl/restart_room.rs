//! `/rs` - restarts the current practice room from the last door it was entered from (see
//! `dungeon::practice::restart_practice_room`). Practice-mode only, see `Server.practice_mode`.

use crate::dungeon::practice;
use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::player::player::Player;
use crate::server::world::World;

pub struct RestartRoom;

impl CommandMetadata for RestartRoom {
    const NAME: &'static str = "rs";

    fn run(world: &mut World, player: &mut Player, _: &[&str]) -> anyhow::Result<Outcome> {
        let server = world.server_mut();

        if !server.practice_mode {
            player.send_message("\u{a7}cThis server wasn't started in practice mode.");
            return Ok(Outcome::Success);
        }

        if let Err(e) = practice::restart_practice_room(server) {
            player.send_message(&format!("\u{a7}c{}", e));
        }

        Ok(Outcome::Success)
    }

    fn arguments(_: &mut World, _: &mut Player) -> Vec<Argument> {
        Vec::new()
    }
}

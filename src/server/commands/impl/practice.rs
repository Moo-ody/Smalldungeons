//! `/practice <room> <door> <secrets> [as]` - loads a single named room in isolation and arms the
//! secret-route timer for the given target secret count (practice-mode only, see
//! `Server.practice_mode`). The trailing `as` is optional and, when present, force-spawns every
//! secret in the room immediately instead of the normal proximity/room-entry gating - omit it for
//! secrets to pop in as you'd see them on a real run. See `dungeon::practice` for the actual
//! room-building/reset/timer logic.

use crate::dungeon::practice;
use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::player::player::Player;
use crate::server::world::World;

const DOOR_CHOICES: &[&str] = &["north", "east", "south", "west", "random"];
/// Tab-complete range only - the real upper bound (a room's actual secret count) is validated
/// against the room's own data in `practice::load_practice_room` once the room is known.
const SECRET_COUNT_CHOICES: std::ops::RangeInclusive<u8> = 1..=10;

pub struct Practice;

impl CommandMetadata for Practice {
    const NAME: &'static str = "practice";

    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
        let server = world.server_mut();

        if !server.practice_mode {
            player.send_message("\u{a7}cThis server wasn't started in practice mode.");
            return Ok(Outcome::Success);
        }

        let room_name = args[0];
        let door_choice = args[1];

        let Ok(target_secrets) = args[2].parse::<u8>() else {
            player.send_message(&format!("\u{a7}c'{}' isn't a valid secret count.", args[2]));
            return Ok(Outcome::Success);
        };

        let instant_secrets = match args.get(3) {
            None => false,
            Some(flag) if flag.eq_ignore_ascii_case("as") => true,
            Some(flag) => {
                player.send_message(&format!("\u{a7}c'{}' isn't valid here - use 'as' to instantly spawn all secrets, or omit it.", flag));
                return Ok(Outcome::Success);
            }
        };

        if let Err(e) = practice::load_practice_room(server, room_name, door_choice, target_secrets, instant_secrets) {
            player.send_message(&format!("\u{a7}c{}", e));
        }

        Ok(Outcome::Success)
    }

    fn arguments(world: &mut World, _: &mut Player) -> Vec<Argument> {
        let server = world.server_mut();
        vec![
            Argument::new("room", true, practice::supported_room_names(server)),
            Argument::new("door", true, DOOR_CHOICES.iter().map(|s| s.to_string()).collect()),
            Argument::new("secrets", true, SECRET_COUNT_CHOICES.map(|n| n.to_string()).collect()),
            Argument::new("as", false, vec!["as".to_string()]),
        ]
    }
}

//! Debug command for exercising F7 score logic (see `dungeon::score`).
//!
//! `death`/`puzzlefail`/`mimic`/`paul` have no real in-game trigger yet - no player death
//! system and no puzzle minigames exist in this codebase - so this is the only way to test
//! those categories end-to-end until those systems are built. `show` prints the live
//! breakdown without mutating anything.
//!
//! NOTE: this command declares exactly one *required* argument (the framework in
//! `commands/mod.rs` rejects anything with a different arg count *silently* - no feedback at
//! all - so `/dscore` with zero args or a typo'd action will do nothing).

use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::player::player::Player;
use crate::server::world::World;

const ACTIONS: &[&str] = &["show", "death", "puzzlefail", "mimic", "paul"];

pub struct DScore;

impl CommandMetadata for DScore {
    const NAME: &'static str = "dscore";

    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
        match args[0] {
            "death" => world.server_mut().dungeon.record_death(),
            "puzzlefail" => world.server_mut().dungeon.record_puzzle_failed(),
            "mimic" => world.server_mut().dungeon.record_mimic_killed(),
            "paul" => world.server_mut().dungeon.set_paul_ezpz(true),
            _ => {}
        }

        let score = &world.server_mut().dungeon.score;
        player.send_message(&format!(
            "\u{a7}7Skill \u{a7}f{} \u{a7}7+ Exploration \u{a7}f{} \u{a7}7+ Speed \u{a7}f{} \u{a7}7+ Bonus \u{a7}f{} \u{a7}7= \u{a7}e{}",
            score.skill(), score.exploration(), score.speed(), score.bonus(), score.total()
        ));

        Ok(Outcome::Success)
    }

    fn arguments(_: &mut World, _: &mut Player) -> Vec<Argument> {
        vec![Argument::new("action", true, ACTIONS.iter().map(|s| s.to_string()).collect())]
    }
}

use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::player::container_ui::UI;
use crate::server::player::player::Player;
use crate::server::world::World;

pub struct Cnc;

impl CommandMetadata for Cnc {
    const NAME: &'static str = "cnc";

    fn run(_: &mut World, player: &mut Player, _: &[&str]) -> anyhow::Result<Outcome> {
        // The "Undersized party!" head is single-use per menu opening - reset the latch every
        // time /cnc (re)opens the menu, not just once per player lifetime.
        player.cnc_undersized_used = false;
        player.open_ui(UI::CncMenu);
        Ok(Outcome::Success)
    }

    fn arguments(_: &mut World, _: &mut Player) -> Vec<Argument> {
        Vec::new()
    }
}

use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::player::container_ui::UI;
use crate::server::player::player::Player;
use crate::server::world::World;

pub struct Mort;

impl CommandMetadata for Mort {
    const NAME: &'static str = "mort";

    fn run(_: &mut World, player: &mut Player, _: &[&str]) -> anyhow::Result<Outcome> { ;
        player.open_ui(UI::MortReadyUpMenu);
        Ok(Outcome::Success)
    }

    fn arguments(world: &mut World, player: &mut Player) -> Vec<Argument> {
        Vec::new()
    }
}

use crate::server::commands::argument::Argument;
use crate::server::commands::outcome::Outcome;
use crate::server::player::player::Player;
use crate::server::world::World;

pub trait CommandMetadata {
    const NAME: &'static str;
    
    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome>;
    fn arguments(world: &mut World, player: &mut Player) -> Vec<Argument>;
}
use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::player::player::Player;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::server::world::World;

pub struct GFS;

impl CommandMetadata for GFS {
    const NAME: &'static str = "gfs";

    // might be able to put like item and count or whatever in the gfs struct, but youd still need to implement a parse method yourself i think   
    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
        let item: String = args[0].to_string();

        let count: i32 = match args[1].parse() {
            Ok(count) => count,
            Err(_) => return Ok(Outcome::Failure(ChatComponentTextBuilder::new("Invalid count.").color(MCColors::Red).build()))
        };

        // todo: this stuff. should be easy when we have a list of items and a method to just give the player items.
        Ok(Outcome::Success)
    }

    fn arguments(world: &mut World, player: &mut Player) -> Vec<Argument> {
        vec![
            Argument {
                name: "item",
                completions: vec!["ender_pearl".to_string()],
            },
            Argument {
                name: "count",
                completions: vec![],
            }
        ]
    }
}
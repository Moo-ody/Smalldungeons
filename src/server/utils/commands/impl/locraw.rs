use crate::net::packets::client_bound::chat::{Chat, CHAT};
use crate::server::player::player::Player;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::commands::argument::Argument;
use crate::server::utils::commands::command::CommandMetadata;
use crate::server::utils::commands::outcome::Outcome;
use crate::server::world::World;

pub struct Locraw;

impl CommandMetadata for Locraw {
    const NAME: &'static str = "locraw";

    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
        player.send_packet(Chat {
            typ: CHAT,
            component: ChatComponentTextBuilder::new(r#"{"server":"mini237V","gametype":"SKYBLOCK","mode":"dungeon","map":"Dungeon"}"#).build(),
        })?;

        Ok(Outcome::Success)
    }

    fn arguments(world: &mut World, player: &mut Player) -> Vec<Argument> {
        Vec::new()
    }
}
use crate::net::packets::client_bound::chat::{Chat, CHAT};
use crate::server::player::player::Player;
use crate::server::player::ui::UI;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::commands::argument::Argument;
use crate::server::utils::commands::command::CommandMetadata;
use crate::server::utils::commands::outcome::Outcome;
use crate::server::world::World;

pub struct Mort;

impl CommandMetadata for Mort {
    const NAME: &'static str = "mort";

    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
        player.send_packet(Chat {
            typ: CHAT,
            component: ChatComponentTextBuilder::new("opening menu").build(),
        })?;
        player.open_ui(UI::MortReadyUpMenu)?;

        Ok(Outcome::Success)
    }

    fn arguments(world: &mut World, player: &mut Player) -> Vec<Argument> {
        Vec::new()
    }
}

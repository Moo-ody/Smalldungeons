use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::player::container_ui::UI::TerminalUI;
use crate::server::player::terminal::{Terminal, TerminalType};
use crate::server::player::player::Player;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::world::World;

pub struct Term;

impl CommandMetadata for Term {
    const NAME: &'static str = "term";

    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {

        if args.is_empty() {
            player.send_message("§cIncorrect usage!");  //temp!
            // since this is nyi...
            return Ok(Outcome::Failure(ChatComponentTextBuilder::new("§cIncorrect usage!").build()))
        }

        let arg = args.get(0).unwrap_or_else(|| &"").to_string().to_lowercase();

        let mut success = false;
        match arg.as_str() {
            "panes" => {
                open_terminal(player, TerminalType::Panes);
                success = true;
            }
            _ => {}
        }

        if success {
            Ok(Outcome::Success)
        } else {
            player.send_message("§cIncorrect usage!"); //temp!
            Ok(Outcome::Failure(ChatComponentTextBuilder::new("§cIncorrect usage!").build()))
        }
    }

    fn arguments(world: &mut World, player: &mut Player) -> Vec<Argument> {
        vec![Argument { name: "type", completions: vec!["panes".to_string()]}]
    }
}

fn open_terminal(player: &mut Player, typ: TerminalType) {
    player.current_terminal = Option::from(Terminal::new(typ));
    player.open_ui(TerminalUI {typ});
}

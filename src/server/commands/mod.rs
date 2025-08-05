use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::commands::r#impl::locraw::Locraw;
use crate::server::commands::r#impl::mort::Mort;
use crate::server::player::player::Player;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::server::world::World;

pub mod command;
pub mod argument;
mod r#impl;
mod outcome;

crate::command_registry! {
    Mort,
    Locraw,
}

impl Command {
    pub fn handle(message: &str, world: &mut World, player: &mut Player) -> anyhow::Result<()> {
        let parts: Vec<&str> = message.split_whitespace().collect();
        if parts.is_empty() {
            // no command given
            return Ok(());
        }

        if let Some(command) = Self::find(parts[0]) {
            let args = &parts[1..];
            let command_args = command.args(world, player);
            if args.len() > command_args.len() {
                let component = ChatComponentTextBuilder::new(format!("Too many arguments! expected: {}, received: {}.", command_args.len(), args.len())).color(MCColors::Red).build();
                // player.send_packet(Chat::new(component, CHAT))?;
                return Ok(());
            }

            let missing_args = &command_args[args.len()..];

            if !missing_args.is_empty() {
                let component =
                    ChatComponentTextBuilder::new("Missing arguments: ")
                        .color(MCColors::Red)
                        .append(
                            ChatComponentTextBuilder::new(missing_args.iter().map(|arg| arg.name).collect::<Vec<&str>>().join(", ")).color(MCColors::Yellow).build()
                        )
                        .build();

                // player.send_packet(Chat::new(component, CHAT))?;
                return Ok(());
            }

            if let Outcome::Failure(component) = command.run(world, player, args)? {
                // player.send_packet(Chat::new(component, CHAT))?;
            }
        } else {
            let unknown_command =
                ChatComponentTextBuilder::new(format!("Unknown command. Type \"/help\" for help. ('{}')", parts[0]))
                    .color(MCColors::Red)
                    .build();
            // player.send_packet(Chat::new(unknown_command, CHAT))?;
        }

        Ok(())
    }
}

#[macro_export]
macro_rules! command_registry {
    {$($name:ident), * $(,)*} => {
        pub enum Command {
            $($name),*
        }
        
        impl Command {
            pub fn list() -> &'static [Command] {
                &[$(Command::$name),*]
            }
            
            pub fn find(name: &str) -> Option<Command> {
                Some(match name {
                    $($name::NAME => Command::$name,)*
                    _ => return None,
                })
            }   
        
            pub fn name(&self) -> &'static str {
                match self {
                    $(Command::$name => $name::NAME),*
                }
            }
            
            pub fn run(&self, world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
                match self {
                    $(Command::$name => $name::run(world, player, args)),*
                }
            }
            
            pub fn args(&self, world: &mut World, player: &mut Player) -> Vec<Argument> {
                match self {
                    $(Command::$name => $name::arguments(world, player)),*
                }
            }
        }
    }
}
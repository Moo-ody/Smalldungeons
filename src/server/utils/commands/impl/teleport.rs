use crate::server::player::player::Player;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::server::utils::commands::argument::Argument;
use crate::server::utils::commands::command::CommandMetadata;
use crate::server::utils::commands::outcome::Outcome;
use crate::server::world::World;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::SendPacket;
use crate::server::utils::dvec3::DVec3;

pub struct SetHome;

impl CommandMetadata for SetHome {
    const NAME: &'static str = "set";

    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
        // Set home position
        player.home_position = Some(player.position);
        let component = ChatComponentTextBuilder::new("Home position set!").color(MCColors::Green).build();
        player.send_packet(crate::net::packets::client_bound::chat::Chat::new(component, crate::net::packets::client_bound::chat::CHAT))?;
        Ok(Outcome::Success)
    }

    fn arguments(world: &mut World, player: &mut Player) -> Vec<Argument> {
        vec![]
    }
}

pub struct TeleportHome;

impl CommandMetadata for TeleportHome {
    const NAME: &'static str = "t";

    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
        // Teleport to home
        if let Some(home_pos) = player.home_position {
            let _ = PositionLook {
                x: home_pos.x,
                y: home_pos.y,
                z: home_pos.z,
                yaw: 0.0,
                pitch: 0.0,
                flags: 24, // keep yaw/pitch
            }.send_packet(player.client_id, &player.network_tx);
            
            Ok(Outcome::Success)
        } else {
            Ok(Outcome::Failure(ChatComponentTextBuilder::new("No home position set! Use /set first.").color(MCColors::Red).build()))
        }
    }

    fn arguments(world: &mut World, player: &mut Player) -> Vec<Argument> {
        vec![]
    }
}

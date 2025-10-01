use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::player::player::Player;
use crate::net::protocol::play::clientbound::PositionLook;
use crate::server::world::World;

pub struct P3;

impl CommandMetadata for P3 {
    const NAME: &'static str = "p3";

    fn run(_: &mut World, player: &mut Player, _: &[&str]) -> anyhow::Result<Outcome> {
        // Teleport to coordinates 100, 116, 40
        let x = 100.0;
        let y = 116.0;
        let z = 40.0;
        
        // Set player position
        player.set_position(x, y, z);
        
        // Send PositionLook packet to client to confirm teleportation
        player.write_packet(&PositionLook {
            x,
            y,
            z,
            yaw: player.yaw,
            pitch: player.pitch,
            flags: 0, // Absolute positioning
        });
        
        // Send confirmation message
        // Note: The actual message sending would need to be implemented
        // For now, we'll just return success
        Ok(Outcome::Success)
    }

    fn arguments(_: &mut World, _: &mut Player) -> Vec<Argument> {
        Vec::new() // No arguments needed
    }
}

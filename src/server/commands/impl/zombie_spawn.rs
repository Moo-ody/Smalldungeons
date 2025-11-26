use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::entity::dungeons_loadouts::dungeons_loadouts;
use crate::server::entity::spawn_equipped::{spawn_equipped_zombie, SpawnOpts};
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::server::world::World;
use crate::net::protocol::play::clientbound::Chat;

pub struct ZombieSpawn;

impl CommandMetadata for ZombieSpawn {
    const NAME: &'static str = "rc_spawn_zombie";

    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
        // Parse arguments: <dx> <dy> <dz> [preset]
        if args.len() < 3 {
            return Ok(Outcome::Failure(
                ChatComponentTextBuilder::new("Usage: /rc_spawn_zombie <dx> <dy> <dz> [preset]")
                    .color(MCColors::Red)
                    .build()
            ));
        }

        // Parse relative coordinates
        let dx = parse_coordinate(args[0]).ok_or_else(|| {
            anyhow::anyhow!("Invalid x coordinate: {}", args[0])
        })?;
        let dy = parse_coordinate(args[1]).ok_or_else(|| {
            anyhow::anyhow!("Invalid y coordinate: {}", args[1])
        })?;
        let dz = parse_coordinate(args[2]).ok_or_else(|| {
            anyhow::anyhow!("Invalid z coordinate: {}", args[2])
        })?;

        // Get preset (default: commander)
        let preset = args.get(3).unwrap_or(&"commander");
        
        // Calculate absolute position
        let base_pos = player.position;
        let absolute_pos = DVec3 {
            x: base_pos.x + dx,
            y: base_pos.y + dy,
            z: base_pos.z + dz,
        };

        // Get equipment preset
        let equipment = match *preset {
            "grunt" => dungeons_loadouts::zombie_grunt(),
            "commander" | _ => dungeons_loadouts::zombie_commander(),
        };

        // Spawn the equipped zombie
        let entity_id = spawn_equipped_zombie(
            world,
            equipment,
            SpawnOpts {
                pos: absolute_pos,
                yaw: 180.0,
                pitch: 0.0,
                hp: Some(3_500_000.0), // Example high HP for testing
                tags: &["dungeons"],
            },
        );

        // Send success message to player
        player.write_packet(&Chat {
            component: ChatComponentTextBuilder::new(
                format!("Spawned equipped zombie (id: {:?}) at {:?}", entity_id, absolute_pos)
            )
            .color(MCColors::Green)
            .build(),
            chat_type: 0,
        });

        Ok(Outcome::Success)
    }

    fn arguments(_world: &mut World, _player: &mut Player) -> Vec<Argument> {
        vec![
            Argument::new("dx", true, vec![]),  // relative x
            Argument::new("dy", true, vec![]),  // relative y  
            Argument::new("dz", true, vec![]),  // relative z
            Argument::new("preset", false, vec!["commander".to_string(), "grunt".to_string()]),
        ]
    }
}

/// Parse a coordinate string that can be a number or a relative coordinate (~number or ~)
fn parse_coordinate(input: &str) -> Option<f64> {
    if input.starts_with('~') {
        if input == "~" {
            // Just ~ means 0 offset
            Some(0.0)
        } else {
            // ~number means relative offset
            input[1..].parse::<f64>().ok()
        }
    } else {
        // Absolute coordinate
        input.parse::<f64>().ok()
    }
}

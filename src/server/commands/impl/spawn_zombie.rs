use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::entity::dungeons_loadouts::dungeons_loadouts;
use crate::server::entity::entity_metadata::EntityVariant;
use crate::server::entity::spawn_equipped::{spawn_equipped_zombie, spawn_following_nametag, SpawnOpts, AISuspended};
use crate::server::player::player::Player;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::server::world::World;
use crate::net::protocol::play::clientbound::Chat;

pub struct SpawnZombie;

impl CommandMetadata for SpawnZombie {
    const NAME: &'static str = "spawn";

    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
        // Check if the command is "/spawn zombie_commander"
        if args.len() < 1 || args[0] != "zombie_commander" {
            return Ok(Outcome::Failure(
                ChatComponentTextBuilder::new("Usage: /spawn zombie_commander")
                    .color(MCColors::Red)
                    .build()
            ));
        }

        // Spawn zombie at player's position
        let spawn_pos = player.position;
        
        // Debug: Show player position
        player.write_packet(&Chat {
            component: ChatComponentTextBuilder::new(
                format!("Player position: {:?}", player.position)
            )
            .color(MCColors::Yellow)
            .build(),
            chat_type: 0,
        });

        // Get the zombie commander equipment 
        let equipment = dungeons_loadouts::zombie_commander();

        // Spawn the equipped zombie
        let entity_id = spawn_equipped_zombie(
            world,
            equipment,
            SpawnOpts {
                pos: spawn_pos,
                yaw: 180.0,
                pitch: 0.0,
                hp: Some(3_500_000.0), // Zombie Commander HP from the goal
                tags: &["dungeons", "starred"],
            },
        );

        // Ensure zombie arms stay down by updating metadata after spawn
        // We need to update both the entity metadata and combat state to ensure consistency
        if let Some((entity, _)) = world.entities.get_mut(&entity_id) {
            // Update the zombie variant to ensure is_attacking is false (arms down)
            if let EntityVariant::Zombie { is_child, is_villager, is_converting, .. } = entity.metadata.variant {
                entity.metadata.variant = EntityVariant::Zombie {
                    is_child,
                    is_villager,
                    is_converting,
                    is_attacking: false, // Ensure arms are down
                };
                // Also ensure AI is disabled to maintain idle pose
                entity.metadata.ai_disabled = true;
            }
        }
        
        // Update combat state to ensure it matches metadata - this is critical
        // because ZombieImpl checks combat state and may override our metadata
        if let Some(combat_state) = world.get_combat_state_mut(entity_id) {
            combat_state.aggressive = false;
            combat_state.swing_ticks = 0;
        }
        
        // Ensure AI remains suspended to prevent automatic arm raising
        world.set_ai_suspended(entity_id, AISuspended { ticks_left: 20 }); // Keep AI suspended longer
        
        // Send metadata update to all players to ensure the change is applied
        world.send_metadata_update(entity_id);

        // Spawn the following nametag
        match spawn_following_nametag(
            world,
            entity_id,
            "§6✰ §cZombie Commander §a3.5M§c❤",
            0.1, // Position armor stand base just above zombie feet (zombie is 1.95 tall)
        ) {
            Ok(nametag_id) => {
                // Final metadata update to ensure arms stay down after all operations
                world.send_metadata_update(entity_id);
                
                // Send success message to player
                player.write_packet(&Chat {
                    component: ChatComponentTextBuilder::new(
                        format!("Spawned Zombie Commander (id: {}) with nametag (id: {}) - arms down: {}", 
                            entity_id, 
                            nametag_id, 
                            if let Some((entity, _)) = world.entities.get(&entity_id) {
                                match &entity.metadata.variant {
                                    EntityVariant::Zombie { is_attacking, .. } => !is_attacking,
                                    _ => false,
                                }
                            } else {
                                false
                            }
                        )
                    )
                    .color(MCColors::Green)
                    .build(),
                    chat_type: 0,
                });
            }
            Err(e) => {
                // Final metadata update even on nametag failure
                world.send_metadata_update(entity_id);
                
                // Send error message to player
                player.write_packet(&Chat {
                    component: ChatComponentTextBuilder::new(
                        format!("Spawned Zombie Commander (id: {}) but failed to create nametag: {}", entity_id, e)
                    )
                    .color(MCColors::Yellow)
                    .build(),
                    chat_type: 0,
                });
            }
        }

        Ok(Outcome::Success)
    }

    fn arguments(_world: &mut World, _player: &mut Player) -> Vec<Argument> {
        vec![
            Argument::new("zombie_commander", true, vec!["zombie_commander".to_string()]),
        ]
    }
}

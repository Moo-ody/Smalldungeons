//! Spawn command for dungeon mobs.
//!
//! **Adding more zombie variants:**  
//! 1. Add a loadout in `server::entity::dungeons_loadouts` (e.g. `zombie_archer()`).  
//! 2. Add the mob type to `SPAWN_MOB_TYPES` and handle it in `mob_config()` (equipment, hp, nametag).  
//!
//! **Adding a different mob type (e.g. Skeleton):**  
//! 1. Add `EntityVariant::Skeleton { ... }` in `entity_metadata.rs` (with correct MC entity id).  
//! 2. Add `spawn_equipped_skeleton()` (and a `SkeletonImpl` if needed) in `spawn_equipped.rs`.  
//! 3. Add a loadout in `dungeons_loadouts` and a branch here that calls the new spawn function.

use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::entity::dungeons_loadouts::dungeons_loadouts;
use crate::server::entity::entity_metadata::EntityVariant;
use crate::server::entity::equipment::Equipment;
use crate::server::entity::spawn_equipped::{spawn_equipped_zombie, spawn_following_nametag, SpawnOpts, AISuspended};
use crate::server::player::player::Player;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::server::world::World;
use crate::net::protocol::play::clientbound::Chat;

/// Mob type identifier for /spawn <mob_type>. Add new entries here and in SPAWN_MOB_TYPES.
const SPAWN_MOB_TYPES: &[&str] = &["zombie_commander", "zombie_grunt", "zombie_custom"];

fn mob_config(mob_type: &str) -> Option<(Equipment, f64, &'static str)> {
    let equipment = match mob_type {
        "zombie_commander" => dungeons_loadouts::zombie_commander(),
        "zombie_grunt" => dungeons_loadouts::zombie_grunt(),
        "zombie_custom" => dungeons_loadouts::zombie_custom(),
        _ => return None,
    };
    let (hp, nametag) = match mob_type {
        "zombie_commander" => (3_500_000.0, "§6✰ §cZombie Commander §a3.5M§c❤"),
        "zombie_grunt" => (100.0, "§7Zombie Grunt §a100§c❤"),
        "zombie_custom" => (500.0, "§dZombie Custom §a500§c❤"),
        _ => return None,
    };
    Some((equipment, hp, nametag))
}

pub struct SpawnZombie;

impl CommandMetadata for SpawnZombie {
    const NAME: &'static str = "spawn";

    fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
        if args.is_empty() {
            return Ok(Outcome::Failure(
                ChatComponentTextBuilder::new(format!("Usage: /spawn <mob_type>. Types: {}", SPAWN_MOB_TYPES.join(", ")))
                    .color(MCColors::Red)
                    .build()
            ));
        }

        let (equipment, hp, nametag_text) = match mob_config(args[0]) {
            Some(c) => c,
            None => {
                return Ok(Outcome::Failure(
                    ChatComponentTextBuilder::new(format!("Unknown mob type '{}'. Try: {}", args[0], SPAWN_MOB_TYPES.join(", ")))
                        .color(MCColors::Red)
                        .build()
                ));
            }
        };

        let spawn_pos = player.position;

        player.write_packet(&Chat {
            component: ChatComponentTextBuilder::new(format!("Player position: {:?}", player.position))
                .color(MCColors::Yellow)
                .build(),
            chat_type: 0,
        });

        let entity_id = spawn_equipped_zombie(
            world,
            equipment,
            SpawnOpts {
                pos: spawn_pos,
                yaw: 180.0,
                pitch: 0.0,
                hp: Some(hp as f32),
                tags: &["dungeons", "starred"],
            },
        );

        if let Some((entity, _)) = world.entities.get_mut(&entity_id) {
            if let EntityVariant::Zombie { is_child, is_villager, is_converting, .. } = entity.metadata.variant {
                entity.metadata.variant = EntityVariant::Zombie {
                    is_child,
                    is_villager,
                    is_converting,
                    is_attacking: false,
                };
                entity.metadata.ai_disabled = true;
            }
        }

        if let Some(combat_state) = world.get_combat_state_mut(entity_id) {
            combat_state.aggressive = false;
            combat_state.swing_ticks = 0;
        }

        world.set_ai_suspended(entity_id, AISuspended { ticks_left: 20 });
        world.send_metadata_update(entity_id);

        match spawn_following_nametag(world, entity_id, nametag_text, 0.1) {
            Ok(nametag_id) => {
                world.send_metadata_update(entity_id);
                player.write_packet(&Chat {
                    component: ChatComponentTextBuilder::new(
                        format!("Spawned {} (id: {}) with nametag (id: {})", args[0], entity_id, nametag_id)
                    )
                    .color(MCColors::Green)
                    .build(),
                    chat_type: 0,
                });
            }
            Err(e) => {
                world.send_metadata_update(entity_id);
                player.write_packet(&Chat {
                    component: ChatComponentTextBuilder::new(
                        format!("Spawned {} (id: {}) but failed to create nametag: {}", args[0], entity_id, e)
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
            Argument::new(
                "mob_type",
                true,
                SPAWN_MOB_TYPES.iter().map(|s| (*s).to_string()).collect(),
            ),
        ]
    }
}

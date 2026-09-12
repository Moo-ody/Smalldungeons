use crate::net::protocol::play::clientbound::{SoundEffect, Particles};
use crate::server::player::player::Player;
use crate::server::utils::sounds::Sounds;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::aabb::AABB;
use crate::server::entity::entity_metadata::EntityVariant;
use crate::net::internal_packets::NetworkThreadMessage;
use tokio::sync::mpsc::UnboundedSender;
use std::f64::consts::PI;

/// Minimum time between Hyperion right-click uses.
const HYPERION_COOLDOWN_SECS: f64 = 0.05;

pub fn on_right_click(player: &mut Player) -> anyhow::Result<()> {
    let now = std::time::Instant::now();
    if let Some(last_used) = player.hyperion_last_used {
        if now.duration_since(last_used).as_secs_f64() < HYPERION_COOLDOWN_SECS {
            return Ok(());
        }
    }
    player.hyperion_last_used = Some(now);

    // Use the exact same teleport logic as ether transmission, but with 10 blocks
    let server = &mut player.server_mut();
    let teleport_result = handle_hyperion_teleport(player, &server.network_tx);

    // Only play sounds if teleport was successful
    if let Ok(Some(dest_pos)) = teleport_result {
        // Always play endermen portal sound on right-click
        let _ = player.write_packet(&SoundEffect {
            sound: Sounds::EndermenPortal.id(),
            volume: 1.0,
            pitch: 1.0,
            pos_x: player.position.x,
            pos_y: player.position.y,
            pos_z: player.position.z,
        });

        // Trigger explosion at destination
        handle_hyperion_explosion(server, dest_pos);
    }

    Ok(())
}

pub(crate) fn handle_hyperion_explosion(server: &mut crate::server::server::Server, explosion_pos: DVec3) {
    const EXPLOSION_RADIUS: f64 = 7.0; // 7 block radius = 13x13x13 area

    // Create explosion AABB (13x13x13 centered at explosion_pos)
    let explosion_aabb = AABB {
        min: DVec3::new(
            explosion_pos.x - EXPLOSION_RADIUS,
            explosion_pos.y - EXPLOSION_RADIUS,
            explosion_pos.z - EXPLOSION_RADIUS,
        ),
        max: DVec3::new(
            explosion_pos.x + EXPLOSION_RADIUS,
            explosion_pos.y + EXPLOSION_RADIUS,
            explosion_pos.z + EXPLOSION_RADIUS,
        ),
    };

    // Play explosion particles and sound for all players
    let explosion_particle = Particles {
        particle_id: 1, // largeexplode
        long_distance: true,
        x: explosion_pos.x as f32,
        y: explosion_pos.y as f32,
        z: explosion_pos.z as f32,
        offset_x: 0.0,
        offset_y: 0.0,
        offset_z: 0.0,
        speed: 0.0,
        count: 0,
    };

    for (_, player) in &mut server.world.players {
        player.write_packet(&explosion_particle);
        player.write_packet(&SoundEffect {
            sound: Sounds::RandomExplode.id(),
            volume: 1.0,
            pitch: 1.0,
            pos_x: explosion_pos.x,
            pos_y: explosion_pos.y,
            pos_z: explosion_pos.z,
        });
    }

    // Check for secret bats and dungeon mobs (zombies with combat state) in explosion range.
    // Spirit Sceptre bats are a separate entity variant and should not be affected.
    let mut bats_to_kill = Vec::new();
    let mut dungeon_mobs_to_kill = Vec::new();

    let secret_bat_ids: std::collections::HashSet<i32> = server
        .dungeon
        .rooms
        .iter()
        .flat_map(|room| room.json_secrets.iter())
        .filter_map(|secret_rc| secret_rc.borrow().bat_entity_id)
        .collect();
    for (entity_id, (entity, _)) in &server.world.entities {
        // Falling block entities should be completely invincible / not affected by Wither Impact.
        if matches!(entity.metadata.variant, EntityVariant::FallingBlock) {
            continue;
        }
        let entity_aabb = AABB {
            min: DVec3::new(entity.position.x - 0.3, entity.position.y, entity.position.z - 0.3),
            max: DVec3::new(entity.position.x + 0.3, entity.position.y + 0.9, entity.position.z + 0.3),
        };
        if !explosion_aabb.intersects(&entity_aabb) {
            continue;
        }
        // Only secret bats die to hyp (secrets)
        if let EntityVariant::Bat { .. } = &entity.metadata.variant {
            if secret_bat_ids.contains(entity_id) {
                bats_to_kill.push(*entity_id);
            }
        }
        // Any dungeon mob dies to hyp, same way as bats - `entity_mob_ai` is present on
        // every dungeon mob regardless of model (zombie/skeleton/enderman/player-NPC), unlike
        // `entity_combat_state` which only melee archetypes register.
        else if server.world.entity_mob_ai.contains_key(entity_id) {
            dungeon_mobs_to_kill.push(*entity_id);
        }
    }

    // Kill bats and mark associated secrets as obtained
    let mut rooms_to_update_map: Vec<usize> = Vec::new();
    for bat_id in bats_to_kill {
        // Find secret associated with this bat and mark it as obtained
        for (room_index, room) in server.dungeon.rooms.iter_mut().enumerate() {
            for secret_rc in &room.json_secrets {
                let mut secret = secret_rc.borrow_mut();
                if let Some(secret_bat_id) = secret.bat_entity_id {
                    if secret_bat_id == bat_id {
                        // Mark secret as obtained and counted
                        if !secret.obtained {
                            secret.obtained = true;
                            if !secret.counted {
                                secret.counted = true;
                                // Increment room's found_secrets count
                                let old_count = room.found_secrets;
                                room.found_secrets = room.found_secrets.saturating_add(1);

                                // Track room for map update if secret count changed and room is entered
                                if old_count != room.found_secrets && room.entered {
                                    rooms_to_update_map.push(room_index);
                                }
                            }
                        }
                        secret.bat_entity_id = None;
                        break;
                    }
                }
            }
        }

        // Play bat death sound
        if let Some((bat_entity, _)) = server.world.entities.get(&bat_id) {
            let bat_pos = bat_entity.position;
            for (_, player) in &mut server.world.players {
                let _ = player.write_packet(&SoundEffect {
                    sound: Sounds::BatDeath.id(),
                    pos_x: bat_pos.x,
                    pos_y: bat_pos.y,
                    pos_z: bat_pos.z,
                    volume: 1.0,
                    pitch: 1.0,
                });
            }
        }

        // Despawn the bat
        server.world.despawn_entity(bat_id);
    }

    // Kill dungeon mobs (zombie commanders etc.) in explosion range - death animation +
    // species-appropriate sound (not always zombie) + despawn, shared with the direct
    // lethal-weapon hit path. King Midas is the one exception: this AOE counts as one weapon
    // hit toward his armor-break/5-hit-kill sequence instead of an instant kill, same as a
    // direct melee hit would (see `ai/combat.rs::apply_king_midas_weapon_hit`).
    for mob_id in dungeon_mobs_to_kill {
        if !crate::server::entity::dungeon_mobs::ai::combat::apply_king_midas_weapon_hit(&mut server.world, mob_id) {
            crate::server::entity::dungeon_mobs::ai::combat::kill_mob(&mut server.world, mob_id);
        }
    }

    // Update map for rooms that had secrets found
    for room_index in rooms_to_update_map {
        server.dungeon.update_map_for_room(room_index);
    }
}

fn handle_hyperion_teleport(
    player: &mut Player,
    _network_tx: &UnboundedSender<NetworkThreadMessage>,
) -> anyhow::Result<Option<DVec3>> {
    const MAX_DISTANCE: f64 = 10.0; // Hyperion has 10 block range

    // Start from eye position
    let mut start = player.position;
    start.y += 1.62;

    // Direction from yaw/pitch (1.8)
    let yaw = player.yaw as f64;
    let pitch = player.pitch as f64;
    let rad_yaw = -yaw.to_radians() - PI;
    let rad_pitch = -pitch.to_radians();
    let f2 = -rad_pitch.cos();
    let dir = DVec3 {
        x: rad_yaw.sin() * f2,
        y: rad_pitch.sin(),
        z: rad_yaw.cos() * f2,
    }.normalize();

    // Swept ray up to MAX_DISTANCE, track last two-high passable cell
    let step_len = 0.25f64;
    let steps = (MAX_DISTANCE / step_len).ceil() as i32;
    let step = DVec3::new(dir.x * step_len, dir.y * step_len, dir.z * step_len);
    let mut last_safe_block: Option<(i32, i32, i32)> = None;
    let mut current = start;

    for step_index in 0..steps {
        current = DVec3::new(current.x + step.x, current.y + step.y, current.z + step.z);
        let bx = current.x.floor() as i32;
        let by = current.y.floor() as i32;
        let bz = current.z.floor() as i32;

        // Same carve-out as ether transmission (`etherwarp::handle_teleport`): ignore the first
        // block raycast, but only if that first block is Iron Bars - every other first-sample
        // block still goes through the normal check below.
        if step_index == 0 && matches!(block_at(player, bx, by, bz), crate::server::block::blocks::Blocks::IronBars) {
            last_safe_block = Some((bx, by, bz));
            continue;
        }

        // We want to stand in this cell: require feet and head passable
        if is_passable_for_transmission(block_at(player, bx, by, bz))
            && is_passable_for_transmission(block_at(player, bx, by + 1, bz)) {
            last_safe_block = Some((bx, by, bz));
            continue;
        } else {
            break; // hit a solid; stop in front
        }
    }

    if let Some((bx, by, bz)) = last_safe_block {
        // Final destination at center of the last safe block
        let dest_x = bx as f64 + 0.5;
        let dest_y = by as f64; // feet at block base; client packet uses absolute feet
        let dest_z = bz as f64 + 0.5;

        // `server_teleport` updates `player.position` immediately (needed for the explosion
        // calculation below) and guards against stale pre-teleport position reports.
        player.server_teleport(DVec3::new(dest_x, dest_y, dest_z), 0.0, 0.0, 24);

        // Return destination position for explosion
        return Ok(Some(DVec3::new(dest_x, dest_y, dest_z)));
    }

    Ok(None)
}

#[inline]
fn block_at(player: &mut Player, x: i32, y: i32, z: i32) -> crate::server::block::blocks::Blocks {
    player.server_mut().world.get_block_at(x, y, z)
}

#[inline]
fn is_passable_for_transmission(block: crate::server::block::blocks::Blocks) -> bool {
    match block {
        crate::server::block::blocks::Blocks::Air
        | crate::server::block::blocks::Blocks::FlowingWater { .. }
        | crate::server::block::blocks::Blocks::StillWater { .. }
        | crate::server::block::blocks::Blocks::FlowingLava { .. }
        | crate::server::block::blocks::Blocks::Lava { .. }
        | crate::server::block::blocks::Blocks::Tallgrass { .. }
        | crate::server::block::blocks::Blocks::Deadbush
        | crate::server::block::blocks::Blocks::Torch { .. }
        | crate::server::block::blocks::Blocks::UnlitRedstoneTorch { .. }
        | crate::server::block::blocks::Blocks::RedstoneTorch { .. }
        | crate::server::block::blocks::Blocks::Redstone { .. }
        | crate::server::block::blocks::Blocks::YellowFlower
        | crate::server::block::blocks::Blocks::RedFlower { .. }
        | crate::server::block::blocks::Blocks::Vine { .. }
        | crate::server::block::blocks::Blocks::Fire
        | crate::server::block::blocks::Blocks::Lilypad
        | crate::server::block::blocks::Blocks::Carpet { .. }
        | crate::server::block::blocks::Blocks::SnowLayer { .. }
        | crate::server::block::blocks::Blocks::Skull { .. }
        | crate::server::block::blocks::Blocks::FlowerPot { .. }
        | crate::server::block::blocks::Blocks::RedstoneComparator { .. }
        | crate::server::block::blocks::Blocks::PoweredRedstoneComparator { .. }
        | crate::server::block::blocks::Blocks::RedstoneRepeater { .. }
        | crate::server::block::blocks::Blocks::PoweredRedstoneRepeater { .. }
        | crate::server::block::blocks::Blocks::Rail { .. }
        | crate::server::block::blocks::Blocks::PoweredRail { .. }
        | crate::server::block::blocks::Blocks::DetectorRail { .. }
        | crate::server::block::blocks::Blocks::DaylightSensor { .. }
        | crate::server::block::blocks::Blocks::InvertedDaylightSensor { .. }
        | crate::server::block::blocks::Blocks::Ladder { .. }
        | crate::server::block::blocks::Blocks::Trapdoor { open: true, .. }
        | crate::server::block::blocks::Blocks::IronTrapdoor { open: true, .. }
        | crate::server::block::blocks::Blocks::SpruceFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::BirchFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::JungleFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::DarkOakFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::AcaciaFenceGate { open: true, .. } => true,
        _ => false,
    }
}

use crate::net::protocol::play::clientbound::{SoundEffect, PositionLook, Particles};
use crate::server::player::player::Player;
use crate::server::utils::sounds::Sounds;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::aabb::AABB;
use crate::server::entity::entity_metadata::EntityVariant;
use crate::net::internal_packets::NetworkThreadMessage;
use tokio::sync::mpsc::UnboundedSender;
use std::f64::consts::PI;

pub fn on_right_click(player: &mut Player) -> anyhow::Result<()> {
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

fn handle_hyperion_explosion(server: &mut crate::server::server::Server, explosion_pos: DVec3) {
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
    
    // Check for bats in explosion range
    let mut bats_to_kill = Vec::new();
    for (entity_id, (entity, _)) in &server.world.entities {
        // Check if entity is a bat
        if let EntityVariant::Bat { .. } = &entity.metadata.variant {
            // Check if bat is within explosion range
            let bat_aabb = AABB {
                min: DVec3::new(entity.position.x - 0.3, entity.position.y, entity.position.z - 0.3),
                max: DVec3::new(entity.position.x + 0.3, entity.position.y + 0.9, entity.position.z + 0.3),
            };
            
            if explosion_aabb.intersects(&bat_aabb) {
                bats_to_kill.push(*entity_id);
            }
        }
    }
    
    // Kill bats and mark associated secrets as obtained
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
                                room.found_secrets = room.found_secrets.saturating_add(1);
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

    for _ in 0..steps {
        current = DVec3::new(current.x + step.x, current.y + step.y, current.z + step.z);
        let bx = current.x.floor() as i32;
        let by = current.y.floor() as i32;
        let bz = current.z.floor() as i32;

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

        player.write_packet(&PositionLook {
            x: dest_x,
            y: dest_y,
            z: dest_z,
            yaw: 0.0,
            pitch: 0.0,
            flags: 24,
        });
        
        // Update player position immediately for explosion calculation
        player.position = DVec3::new(dest_x, dest_y, dest_z);
        
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
        | crate::server::block::blocks::Blocks::Carpet { .. } => true,
        _ => false,
    }
}
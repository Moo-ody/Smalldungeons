use crate::net::protocol::play::clientbound::{EntityVelocity, SoundEffect, Particles};
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::var_int::VarInt;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::{ClientId, Player};
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use anyhow::Result;
use std::f64::consts::PI;

// Core constants from the guide
const TPS: f64 = 20.0; // Minecraft runs at 20 ticks per second
const PROJECTILE_SPEED: f64 = 20.0; // blocks per second
const EXPLOSION_RADIUS: f64 = 2.5;
const KNOCKBACK_STRENGTH: f64 = 0.9;
const SPAWN_OFFSET: f64 = 0.30; // blocks forward from player eye

/// Explosive projectile for Bonzo Staff
pub struct BonzoProjectileImpl {
    thrower_client_id: ClientId,
    velocity_per_tick: DVec3,
    explosion_radius: f64,
    base_knockback: f64,
    vertical_boost: f64,
    max_ticks: u32,
}

impl BonzoProjectileImpl {
    pub fn new(thrower_client_id: ClientId, direction: DVec3, speed_bps: f64) -> Self {
        // Convert blocks per second to blocks per tick
        let velocity_per_tick = DVec3::new(
            direction.x * (speed_bps / TPS),
            direction.y * (speed_bps / TPS),
            direction.z * (speed_bps / TPS),
        );
        
        Self {
            thrower_client_id,
            velocity_per_tick,
            explosion_radius: EXPLOSION_RADIUS,
            base_knockback: KNOCKBACK_STRENGTH,
            vertical_boost: 2.0,   // Stronger vertical boost for knockback
            max_ticks: 200, // 10 seconds max lifetime
        }
    }
}

impl EntityImpl for BonzoProjectileImpl {
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Set initial velocity for smooth projectile animation
        for _player in entity.world_mut().players.values() {
            let _ = packet_buffer.write_packet(&EntityVelocity {
                entity_id: VarInt(entity.id),
                velocity_x: (self.velocity_per_tick.x * 8000.0) as i16,
                velocity_y: (self.velocity_per_tick.y * 8000.0) as i16,
                velocity_z: (self.velocity_per_tick.z * 8000.0) as i16,
            });
        }
        entity.velocity = self.velocity_per_tick;
    }

    fn tick(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Check lifetime limit
        if entity.ticks_existed >= self.max_ticks {
            entity.world_mut().despawn_entity(entity.id);
            return;
        }

        // Store previous position for collision detection
        let prev_pos = entity.position;
        
        // Calculate next position
        let next_pos = DVec3::new(
            entity.position.x + self.velocity_per_tick.x,
            entity.position.y + self.velocity_per_tick.y,
            entity.position.z + self.velocity_per_tick.z,
        );

        // Perform segment raycast (prev -> next)
        if let Some(impact_point) = self.segment_raycast(entity, prev_pos, next_pos) {
            println!("BONZO PROJECTILE HIT WALL at ({:.2}, {:.2}, {:.2})", 
                impact_point.x, impact_point.y, impact_point.z);
            self.explode(entity, packet_buffer, impact_point);
            entity.world_mut().despawn_entity(entity.id);
            return;
        }

        // No collision - commit to next position
        entity.position = next_pos;
        entity.velocity = self.velocity_per_tick;
    }
}

impl BonzoProjectileImpl {
    /// Segment raycast from prev to next position
    fn segment_raycast(&self, entity: &Entity, prev_pos: DVec3, next_pos: DVec3) -> Option<DVec3> {
        let direction = next_pos - prev_pos;
        let distance = direction.distance_to(&DVec3::ZERO);
        
        if distance == 0.0 {
            return None;
        }

        let normalized_dir = DVec3::new(
            direction.x / distance,
            direction.y / distance,
            direction.z / distance,
        );
        
        // Use small step size for accurate collision detection
        let step_size = 0.05;
        let steps = (distance / step_size).ceil() as i32;
        
        for i in 0..=steps {
            let t = (i as f64) / (steps as f64);
            let check_pos = prev_pos + DVec3::new(
                normalized_dir.x * (t * distance),
                normalized_dir.y * (t * distance),
                normalized_dir.z * (t * distance),
            );
            
            let block_x = check_pos.x.floor() as i32;
            let block_y = check_pos.y.floor() as i32;
            let block_z = check_pos.z.floor() as i32;
            
            let block = entity.world_mut().get_block_at(block_x, block_y, block_z);
            
            if !is_block_passable_for_projectile(block) {
                return Some(check_pos);
            }
        }
        
        None
    }

    /// Create explosion effects and apply knockback
    fn explode(&self, entity: &Entity, packet_buffer: &mut PacketBuffer, explosion_center: DVec3) {
        let world = entity.world_mut();
        
        println!("BONZO EXPLOSION at ({:.2}, {:.2}, {:.2}) with radius {:.2}", 
            explosion_center.x, explosion_center.y, explosion_center.z, self.explosion_radius);
        println!("BONZO EXPLOSION: Found {} players to check for knockback", world.players.len());
        
        // Play explosion sounds
        for _player in world.players.values() {
            let _ = packet_buffer.write_packet(&SoundEffect {
                sound: Sounds::FireworksBlast.id(),
                pos_x: explosion_center.x,
                pos_y: explosion_center.y,
                pos_z: explosion_center.z,
                volume: 20.0,
                pitch: 0.99,
            });
            let _ = packet_buffer.write_packet(&SoundEffect {
                sound: Sounds::FireworksTwinkle.id(),
                pos_x: explosion_center.x,
                pos_y: explosion_center.y,
                pos_z: explosion_center.z,
                volume: 20.0,
                pitch: 0.99,
            });
        }

        // Generate explosion particles in spherical pattern
        self.generate_explosion_particles(packet_buffer, explosion_center);
        
        // Apply knockback to nearby players
        self.apply_knockback_to_players(world, explosion_center);
    }

    /// Generate particles in spherical explosion pattern
    fn generate_explosion_particles(&self, packet_buffer: &mut PacketBuffer, center: DVec3) {
        let particle_count = 20;
        
        for _ in 0..particle_count {
            // Generate random point on sphere
            let theta = rand::random::<f64>() * 2.0 * PI;
            let phi = (2.0 * rand::random::<f64>() - 1.0).acos();
            let r = rand::random::<f64>() * self.explosion_radius;
            
            let x = center.x + r * phi.sin() * theta.cos();
            let y = center.y + r * phi.cos();
            let z = center.z + r * phi.sin() * theta.sin();
            
            // Spawn explosion particle
            let _ = packet_buffer.write_packet(&Particles {
                particle_id: 0, // Explosion particle
                long_distance: false,
                x: x as f32,
                y: y as f32,
                z: z as f32,
                offset_x: 0.0,
                offset_y: 0.0,
                offset_z: 0.0,
                speed: 0.0,
                count: 1,
            });
        }
    }

    /// Apply knockback to players within explosion radius
    fn apply_knockback_to_players(&self, world: &mut crate::server::world::World, explosion_center: DVec3) {
        let explosion_range = 9.0; // blocks
        
        for (_client_id, player) in world.players.iter_mut() {
            // Use player center position (feet + 0.9 blocks) for more stable knockback
            let player_center = DVec3::new(
                player.position.x,
                player.position.y + 0.9,  // Player center height
                player.position.z,
            );
            
            let distance = explosion_center.distance_to(&player_center);
            
            if distance < explosion_range {
                // Calculate knockback direction (from explosion to player)
                let mut knockback_dir = (player_center - explosion_center);
                
                // Apply distance-based minimum knockback to prevent tiny explosions from having huge effects
                let min_distance = 0.5; // Minimum effective distance
                if knockback_dir.distance_to(&DVec3::ZERO) < min_distance {
                    // If explosion is very close, use player's look direction for knockback
                    let look_dir = look_dir_minecraft(player.yaw, player.pitch);
                    knockback_dir = DVec3::new(look_dir.x, look_dir.y, look_dir.z);
                } else {
                    knockback_dir = knockback_dir.normalize();
                }
                
                // Apply knockback with distance attenuation
                let attenuation = (1.0 - (distance / explosion_range)).max(0.1); // Minimum 10% knockback
                let horizontal_strength = 1.5 * attenuation; // Stronger horizontal knockback
                let vertical_strength = 0.6 * attenuation;   // Reduced vertical knockback
                
                let final_knockback = DVec3::new(
                    knockback_dir.x * horizontal_strength,
                    knockback_dir.y * vertical_strength,  // Reduced vertical
                    knockback_dir.z * horizontal_strength,
                );
                
                // Debug logging
                println!("BONZO KNOCKBACK: distance={:.2}, explosion=({:.2}, {:.2}, {:.2}), player=({:.2}, {:.2}, {:.2}), dir=({:.2}, {:.2}, {:.2}), knockback=({:.2}, {:.2}, {:.2})", 
                    distance, explosion_center.x, explosion_center.y, explosion_center.z,
                    player_center.x, player_center.y, player_center.z,
                    knockback_dir.x, knockback_dir.y, knockback_dir.z,
                    final_knockback.x, final_knockback.y, final_knockback.z);
                
                // Send EntityVelocity packet to apply the knockback
                player.write_packet(&EntityVelocity {
                    entity_id: VarInt(player.entity_id),
                    velocity_x: (final_knockback.x * 8000.0).clamp(-32767.0, 32767.0) as i16,
                    velocity_y: (final_knockback.y * 8000.0).clamp(-32767.0, 32767.0) as i16,
                    velocity_z: (final_knockback.z * 8000.0).clamp(-32767.0, 32767.0) as i16,
                });
                
                player.flush_packets();
            }
        }
    }
}

/// Check if a block is passable for projectiles
#[inline]
fn is_block_passable_for_projectile(block: Blocks) -> bool {
    match block {
        Blocks::Air
        | Blocks::FlowingWater { .. }
        | Blocks::StillWater { .. }
        | Blocks::FlowingLava { .. }
        | Blocks::Lava { .. }
        | Blocks::Tallgrass { .. }
        | Blocks::Deadbush
        | Blocks::Torch { .. }
        | Blocks::UnlitRedstoneTorch { .. }
        | Blocks::RedstoneTorch { .. }
        | Blocks::Redstone { .. }
        | Blocks::YellowFlower
        | Blocks::RedFlower { .. }
        | Blocks::Vine { .. }
        | Blocks::Fire
        | Blocks::Lilypad
        | Blocks::Carpet { .. }
        | Blocks::SnowLayer { .. }
        | Blocks::Skull { .. }
        | Blocks::FlowerPot { .. }
        | Blocks::RedstoneComparator { .. }
        | Blocks::PoweredRedstoneComparator { .. }
        | Blocks::RedstoneRepeater { .. }
        | Blocks::PoweredRedstoneRepeater { .. }
        | Blocks::Rail { .. }
        | Blocks::PoweredRail { .. }
        | Blocks::DetectorRail { .. }
        | Blocks::DaylightSensor { .. }
        | Blocks::InvertedDaylightSensor { .. }
        | Blocks::Ladder { .. }
        | Blocks::Trapdoor { open: true, .. }
        | Blocks::IronTrapdoor { open: true, .. }
        | Blocks::SpruceFenceGate { open: true, .. }
        | Blocks::BirchFenceGate { open: true, .. }
        | Blocks::JungleFenceGate { open: true, .. }
        | Blocks::DarkOakFenceGate { open: true, .. }
        | Blocks::AcaciaFenceGate { open: true, .. } => true,
        _ => false,
    }
}

/// Handle right-click with Bonzo Staff to spawn explosive projectile
pub fn on_right_click(player: &mut Player) -> Result<()> {
    // Play ghast moan sound immediately on right-click
    player.write_packet(&SoundEffect {
        sound: Sounds::GhastMoan.id(),
        pos_x: player.position.x,
        pos_y: player.position.y,
        pos_z: player.position.z,
        volume: 1.0,
        pitch: 1.43,
    });
    
    let eye_height = 1.62; // player eye height in blocks
    let eye_pos = DVec3::new(
        player.position.x,
        player.position.y + eye_height,
        player.position.z,
    );
    
    // MC-correct look direction calculation
    let direction = look_dir_minecraft(player.yaw, player.pitch);
    
    // Calculate spawn position with forward offset to avoid self-collision
    let spawn_pos = eye_pos + DVec3::new(
        direction.x * SPAWN_OFFSET,
        direction.y * SPAWN_OFFSET,
        direction.z * SPAWN_OFFSET,
    );

    // Ping compensation: delay_ticks = round(ping_ms / 50)
    let delay_ticks = ((player.ping as f64) / 50.0).round() as u32;
    
    if delay_ticks == 0 {
        // Spawn immediately for low ping
        player.world_mut().spawn_entity(
            spawn_pos,
            EntityMetadata::new(EntityVariant::BonzoProjectile),
            BonzoProjectileImpl::new(player.client_id, direction, PROJECTILE_SPEED),
        )?;
    } else {
        // Schedule delayed spawn for ping compensation
        let server = player.server_mut();
        let client_id = player.client_id;
        let spawn_pos_clone = spawn_pos;
        let direction_clone = direction;
        
        server.schedule(delay_ticks, move |server| {
            if let Some(player) = server.world.players.get_mut(&client_id) {
                let _ = player.world_mut().spawn_entity(
                    spawn_pos_clone,
                    EntityMetadata::new(EntityVariant::BonzoProjectile),
                    BonzoProjectileImpl::new(client_id, direction_clone, PROJECTILE_SPEED),
                );
            }
        });
    }

    Ok(())
}

/// Converts Minecraft yaw/pitch (in degrees) to a normalized direction vector
/// 
/// Minecraft conventions:
/// - Yaw 0° = +Z (forward)
/// - Yaw 90° = -X (right)
/// - Yaw -90° = +X (left)
/// - Pitch 0° = horizontal
/// - Pitch 90° = straight down (negative Y)
/// - Pitch -90° = straight up (positive Y)
#[inline]
fn look_dir_minecraft(yaw_deg: f32, pitch_deg: f32) -> DVec3 {
    let yaw = yaw_deg as f64 * PI / 180.0;
    let pitch = pitch_deg as f64 * PI / 180.0;
    
    let cos_pitch = pitch.cos();
    let sin_pitch = pitch.sin();
    let cos_yaw = yaw.cos();
    let sin_yaw = yaw.sin();
    
    DVec3::new(
        -sin_yaw * cos_pitch,  // X: +yaw right -> -X
        -sin_pitch,            // Y: pitch down -> negative Y
        cos_yaw * cos_pitch    // Z: forward is +Z
    ).normalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_look_directions() {
        let dir = look_dir_minecraft(0.0, 0.0);
        assert!((dir.x - 0.0).abs() < 0.001);
        assert!((dir.y - 0.0).abs() < 0.001);
        assert!((dir.z - 1.0).abs() < 0.001); // Forward
        
        let dir = look_dir_minecraft(90.0, 0.0);
        assert!((dir.x - (-1.0)).abs() < 0.001); // Right
        assert!((dir.y - 0.0).abs() < 0.001);
        assert!((dir.z - 0.0).abs() < 0.001);
        
        let dir = look_dir_minecraft(0.0, 90.0);
        assert!((dir.x - 0.0).abs() < 0.001);
        assert!((dir.y - (-1.0)).abs() < 0.001); // Down
        assert!((dir.z - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_projectile_speed() {
        let direction = DVec3::new(0.0, 0.0, 1.0); // Forward
        let projectile = BonzoProjectileImpl::new(0, direction, PROJECTILE_SPEED);
        
        // After 20 ticks (1 second), should travel 20 blocks
        let expected_velocity = DVec3::new(0.0, 0.0, PROJECTILE_SPEED / TPS);
        assert!((projectile.velocity_per_tick.z - expected_velocity.z).abs() < 0.001);
    }
}

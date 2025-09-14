use crate::net::protocol::play::clientbound::{EntityVelocity, SoundEffect, Particles, PositionLook};
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

const TPS: f64 = 20.0;
const SPEED_BPS: f64 = 20.0; // blocks per second

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
            explosion_radius: 8.0, // Larger explosion radius
            base_knockback: 4.0,   // Stronger base knockback
            vertical_boost: 3.0,   // Stronger vertical boost
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

        // Generate explosion particles
        self.generate_explosion_particles(packet_buffer, explosion_center);
        
        // Apply knockback to nearby players
        self.apply_knockback_to_players(world, explosion_center);
    }

    /// Generate explosion particles in a spherical pattern
    fn generate_explosion_particles(&self, packet_buffer: &mut PacketBuffer, center: DVec3) {
        const PARTICLE_COUNT: i32 = 50;
        
        for _ in 0..PARTICLE_COUNT {
            // Generate spherical coordinates
            let theta = (rand::random::<f64>() * 2.0 * PI) as f32;
            let phi = (rand::random::<f64>() * PI) as f32;
            let r = rand::random::<f32>() * self.explosion_radius as f32;
            
            // Convert to Cartesian coordinates
            let x = center.x + (r * phi.sin() * theta.cos()) as f64;
            let y = center.y + (r * phi.cos()) as f64;
            let z = center.z + (r * phi.sin() * theta.sin()) as f64;
            
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
        println!("Checking knockback for {} players", world.players.len());
        for (_client_id, player) in world.players.iter_mut() {
            // Use eye position for knockback calculation
            let eye_height = 1.62;
            let eye_pos = DVec3::new(
                player.position.x,
                player.position.y + eye_height,
                player.position.z,
            );
            
            let distance = eye_pos.distance_to(&explosion_center);
            
            println!("Player at distance {:.2} from explosion (radius: {:.2})", distance, self.explosion_radius);
            
            if distance < self.explosion_radius {
                // Calculate direction from explosion center to player's eyes
                let direction_to_player = (eye_pos - explosion_center).normalize();
                
                // Distance attenuation: clamp(1 - d/radius, 0..1)
                let attenuation = (1.0 - (distance / self.explosion_radius)).max(0.0).min(1.0);
                
                // Knockback = (direction_to_player × base_strength × attn) with vertical boost
                let horizontal_knockback = DVec3::new(
                    direction_to_player.x * self.base_knockback * attenuation,
                    direction_to_player.y * self.base_knockback * attenuation,
                    direction_to_player.z * self.base_knockback * attenuation,
                );
                
                // Add vertical boost scaled by attenuation
                let vertical_boost = DVec3::new(0.0, self.vertical_boost * attenuation, 0.0);
                
                let total_knockback = horizontal_knockback + vertical_boost;
                
                // Debug output
                println!("Knockback: distance={:.2}, attn={:.2}, total=({:.2}, {:.2}, {:.2})", 
                    distance, attenuation, total_knockback.x, total_knockback.y, total_knockback.z);
                
                // Apply knockback by directly moving the player (more reliable)
                let new_position = player.position + total_knockback;
                
                // Update player position on server
                player.position = new_position;
                
                // Send PositionLook packet to move the player
                player.write_packet(&PositionLook {
                    x: new_position.x,
                    y: new_position.y,
                    z: new_position.z,
                    yaw: player.yaw,
                    pitch: player.pitch,
                    flags: 0, // Set absolute position
                });
                
                // Also try EntityVelocity as backup
                player.write_packet(&EntityVelocity {
                    entity_id: VarInt(player.entity_id),
                    velocity_x: (total_knockback.x * 4000.0).clamp(-3.9, 3.9) as i16,
                    velocity_y: (total_knockback.y * 4000.0).clamp(-3.9, 3.9) as i16,
                    velocity_z: (total_knockback.z * 4000.0).clamp(-3.9, 3.9) as i16,
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
    
    // Small forward offset to avoid self-collision (0.25-0.35 blocks)
    let spawn_pos = eye_pos + DVec3::new(
        direction.x * 0.30,
        direction.y * 0.30,
        direction.z * 0.30,
    );

    // TODO: Add ping compensation here
    // delay_ticks = round(ping_ms / 50)
    // For now, spawn immediately

    player.world_mut().spawn_entity(
        spawn_pos,
        EntityMetadata::new(EntityVariant::BonzoProjectile),
        BonzoProjectileImpl::new(player.client_id, direction, SPEED_BPS),
    )?;

    Ok(())
}

/// MC-correct look direction calculation
/// Conventions: forward is +Z, yaw=0 faces +Z, yaw=90 faces -X, yaw=-90 faces +X, pitch positive means looking down
#[inline]
fn look_dir_minecraft(yaw_deg: f32, pitch_deg: f32) -> DVec3 {
    let yaw = yaw_deg as f64 * PI / 180.0;
    let pitch = pitch_deg as f64 * PI / 180.0;
    let cp = pitch.cos();
    let dir = DVec3::new(
        -yaw.sin() * cp,   // +yaw right -> -X (MC)
        -pitch.sin(),      // pitch down positive -> negative Y in world-up
        yaw.cos() * cp     // forward is +Z
    );
    dir.normalize()
}

use crate::net::protocol::play::clientbound::{EntityVelocity, PositionLook, SoundEffect};
use crate::net::var_int::VarInt;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use std::f64::consts::PI;

// Bonzo Staff constants based on Hypixel SkyBlock specifications
const PROJECTILE_SPEED: f64 = 12.0; // blocks/sec (community says "slow", starting at 12)
const EXPLOSION_RADIUS: f64 = 2.0; // blocks AOE
const SPAWN_OFFSET: f64 = 0.30; // blocks forward to avoid self-hit
const MANA_COST: i32 = 90; // current mana cost
const MAX_FIRE_RATE: f64 = 4.0; // shots per second
const CORPSE_COLLISION_WINDOW: u32 = 10; // ticks (0.5 seconds)

/// Bonzo Staff projectile implementation
pub struct BonzoStaffProjectile {
    pub thrower_client_id: u32,
    pub velocity: DVec3,
    pub ticks_existed: u32,
}

impl BonzoStaffProjectile {
    pub fn new(thrower_client_id: u32, direction: DVec3) -> Self {
        Self {
            thrower_client_id,
            velocity: direction * PROJECTILE_SPEED,
            ticks_existed: 0,
        }
    }
}

impl EntityImpl for BonzoStaffProjectile {
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Set initial velocity for smooth projectile animation
        for player in entity.world_mut().players.values() {
            let _ = packet_buffer.write_packet(&EntityVelocity {
                entity_id: VarInt(entity.id),
                velocity_x: (self.velocity.x * 8000.0) as i16,
                velocity_y: (self.velocity.y * 8000.0) as i16,
                velocity_z: (self.velocity.z * 8000.0) as i16,
            });
        }
    }

    fn tick(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        self.ticks_existed += 1;
        
        // Basic projectile physics - straight line movement
        let current_pos = entity.position;
        let new_pos = DVec3::new(
            current_pos.x + self.velocity.x / 20.0, // 20 TPS scaling
            current_pos.y + self.velocity.y / 20.0,
            current_pos.z + self.velocity.z / 20.0,
        );

        // Check for collisions using segment raycast
        let collision = self.check_collision(entity, current_pos, new_pos);
        
        if let Some((collision_pos, collision_type)) = collision {
            // Explode on collision
            self.explode(entity, collision_pos, packet_buffer);
            entity.world_mut().despawn_entity(entity.id);
            return;
        }

        // Lifetime limit to avoid lingering forever
        if self.ticks_existed > 200 { // 10 seconds max
            entity.world_mut().despawn_entity(entity.id);
            return;
        }

        // Update position
        entity.position = new_pos;
    }
}

impl BonzoStaffProjectile {
    /// Check for collision using segment raycast
    fn check_collision(&self, entity: &Entity, start: DVec3, end: DVec3) -> Option<(DVec3, CollisionType)> {
        let world = entity.world_mut();
        let max_delta = (end.x - start.x).abs().max((end.y - start.y).abs()).max((end.z - start.z).abs());
        let steps = (max_delta / 0.2).ceil().max(1.0) as i32;
        let step = DVec3::new(
            (end.x - start.x) / steps as f64,
            (end.y - start.y) / steps as f64,
            (end.z - start.z) / steps as f64,
        );

        for i in 0..=steps {
            let current = DVec3::new(
                start.x + step.x * i as f64,
                start.y + step.y * i as f64,
                start.z + step.z * i as f64,
            );

            let block_x = current.x.floor() as i32;
            let block_y = current.y.floor() as i32;
            let block_z = current.z.floor() as i32;

            // Check block collision
            let block = world.get_block_at(block_x, block_y, block_z);
            if !self.is_block_passable(block) {
                return Some((current, CollisionType::Block));
            }

            // Check mob collision
            if let Some((mob_entity, _)) = world.entities.get(&self.find_nearby_mob(world, current)) {
                if mob_entity.id != entity.id {
                    return Some((current, CollisionType::Mob));
                }
            }

            // Check corpse collision (recently dead mobs)
            if self.is_corpse_collision(world, current) {
                return Some((current, CollisionType::Corpse));
            }
        }

        None
    }

    /// Check if block is passable for projectile
    fn is_block_passable(&self, block: Blocks) -> bool {
        matches!(block, 
            Blocks::Air
            | Blocks::FlowingWater { .. }
            | Blocks::StillWater { .. }
            | Blocks::FlowingLava { .. }
            | Blocks::Lava { .. }
            | Blocks::Tallgrass { .. }
            | Blocks::Deadbush
            | Blocks::Torch { .. }
            | Blocks::Fire
            | Blocks::Lilypad
        )
    }

    /// Find nearby mob for collision detection
    fn find_nearby_mob(&self, world: &World, pos: DVec3) -> Option<i32> {
        for (entity_id, (entity, _)) in &world.entities {
            let distance = entity.position.distance(pos);
            if distance < 0.5 { // Projectile hitbox
                return Some(*entity_id);
            }
        }
        None
    }

    /// Check for corpse collision (recently dead mobs)
    fn is_corpse_collision(&self, world: &World, pos: DVec3) -> bool {
        // This would need to track recently dead mobs
        // For now, return false - can be implemented later
        false
    }

    /// Explode and apply knockback
    fn explode(&self, entity: &Entity, explosion_pos: DVec3, packet_buffer: &mut PacketBuffer) {
        let world = entity.world_mut();
        
        // Play explosion sound
        for (_, player) in &mut world.players {
            let _ = player.write_packet(&SoundEffect {
                sound: crate::server::utils::sounds::Sounds::Explode.id(),
                pos_x: explosion_pos.x,
                pos_y: explosion_pos.y,
                pos_z: explosion_pos.z,
                volume: 4.0,
                pitch: 1.0,
            });
        }

        // Apply knockback to all players within explosion radius
        for (_, player) in &mut world.players {
            self.apply_knockback(player, explosion_pos);
        }
    }

    /// Apply knockback to a player within explosion radius
    fn apply_knockback(&self, player: &mut Player, explosion_pos: DVec3) {
        let explosion_range = EXPLOSION_RADIUS;
        let player_center_offset = DVec3::new(0.0, 0.9, 0.0); // Player center height
        
        // Calculate distance from explosion to player center
        let player_center = player.position + player_center_offset;
        let distance = explosion_pos.distance(player_center);
        
        if distance < explosion_range {
            // Attenuation factor (linear falloff)
            let attenuation = (1.0 - (distance / explosion_range)).max(0.1);
            
            // Knockback strength (different for horizontal vs vertical)
            let horizontal_strength = 1.5 * attenuation;  // 1.5 blocks max
            let vertical_strength = 0.6 * attenuation;    // 0.6 blocks max
            
            // Calculate knockback direction
            let knockback_dir = (player_center - explosion_pos).normalized();
            
            // Final knockback vector
            let final_knockback = DVec3::new(
                knockback_dir.x * horizontal_strength,
                knockback_dir.y * vertical_strength,
                knockback_dir.z * horizontal_strength,
            );
            
            // Convert to velocity packet format
            let velocity_x = (final_knockback.x * 8000.0) as i16;
            let velocity_y = (final_knockback.y * 8000.0) as i16;
            let velocity_z = (final_knockback.z * 8000.0) as i16;
            
            // Send velocity packet to player
            player.write_packet(&EntityVelocity {
                entity_id: VarInt(player.entity_id),
                velocity_x,
                velocity_y,
                velocity_z,
            });
        }
    }
}

#[derive(Debug, Clone)]
enum CollisionType {
    Block,
    Mob,
    Corpse,
}

/// Bonzo Staff item implementation
pub struct BonzoStaff {
    pub last_shot_tick: u64,
}

impl BonzoStaff {
    pub fn new() -> Self {
        Self {
            last_shot_tick: 0,
        }
    }

    /// Check if player can shoot (rate limiting)
    pub fn can_shoot(&self, current_tick: u64) -> bool {
        let ticks_per_shot = (20.0 / MAX_FIRE_RATE) as u64; // 20 TPS / 4 shots/sec = 5 ticks
        current_tick - self.last_shot_tick >= ticks_per_shot
    }

    /// Shoot a balloon projectile
    pub fn shoot(&mut self, player: &mut Player, world: &mut World) -> anyhow::Result<()> {
        let current_tick = world.tick_count;
        
        // Check rate limiting
        if !self.can_shoot(current_tick) {
            return Ok(()); // Silently ignore if too fast
        }

        // Calculate direction from player's look vector
        let yaw_rad = (player.yaw as f64).to_radians();
        let pitch_rad = (player.pitch as f64).to_radians();
        
        let direction = DVec3::new(
            -pitch_rad.cos() * yaw_rad.sin(),
            -pitch_rad.sin(),
            pitch_rad.cos() * yaw_rad.cos(),
        ).normalized();

        // Spawn position with forward offset to avoid self-hit
        let eye_height = 1.62; // Player eye height
        let spawn_pos = DVec3::new(
            player.position.x + direction.x * SPAWN_OFFSET,
            player.position.y + eye_height + direction.y * SPAWN_OFFSET,
            player.position.z + direction.z * SPAWN_OFFSET,
        );

        // Create and spawn projectile
        let projectile = BonzoStaffProjectile::new(player.client_id, direction);
        
        world.spawn_entity(
            spawn_pos,
            EntityMetadata::new(EntityVariant::FallingBlock), // Use falling block as projectile
            projectile,
        )?;

        // Update last shot time
        self.last_shot_tick = current_tick;

        Ok(())
    }
}


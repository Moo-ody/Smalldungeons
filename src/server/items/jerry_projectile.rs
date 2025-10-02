use crate::net::protocol::play::clientbound::{EntityMoveRotate, EntityVelocity};
use crate::net::var_int::VarInt;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::player::player::ClientId;
use crate::server::utils::dvec3::DVec3;
use crate::server::block::blocks::Blocks;
use crate::net::packets::packet_buffer::PacketBuffer;

// Core constants matching Java implementation
const TPS: f64 = 20.0; // Minecraft runs at 20 ticks per second
const PROJECTILE_SPEED: f64 = 20.0; // blocks per second
const MAX_LIFETIME_TICKS: u32 = 60; // 3 seconds (20 * 3) like Java
const JERRY_RANGE: f64 = 4.7; // Range for knockback (works at 35.7xx when wall is at 31)
const JERRY_VELO: f64 = 0.55; // Vertical velocity for knockback (from DungeonSim.config.jerryVelo = 0.55F)
const JERRY_HORIZONTAL_MULT: f64 = 0.275; // Horizontal multiplier (jerryVelo / 2 = 0.55 / 2 = 0.275)

/// Jerry-Chine Gun projectile implementation matching Java version
pub struct JerryProjectileImpl {
    thrower_client_id: ClientId,
    velocity_per_tick: DVec3,
    set_dead: bool, // Track if already scheduled for death like Java
}

impl JerryProjectileImpl {
    pub fn new(thrower_client_id: ClientId, direction: DVec3, speed_bps: f64) -> Self {
        let velocity_per_tick = DVec3::new(
            direction.x * (speed_bps / TPS),
            direction.y * (speed_bps / TPS),
            direction.z * (speed_bps / TPS),
        );
        Self {
            thrower_client_id,
            velocity_per_tick,
            set_dead: false, // Start alive
        }
    }
    
    fn set_velocity(&self, entity: &mut Entity) {
        entity.velocity = self.velocity_per_tick;
    }
    
    fn check_collision(&self, entity: &Entity) -> bool {
        let world = entity.world_mut();
        let pos = entity.position;
        
        // Check block collision at current position
        let block = world.get_block_at(
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        );
        
        // Check if block is solid (not passable for projectiles)
        !is_block_passable_for_projectile(block)
    }
    
    fn schedule_knockback_for_next_tick(&self, entity: &Entity) {
        let client_id = self.thrower_client_id;
        let player_pos = entity.world_mut().players.get(&client_id).map_or(DVec3::ZERO, |p| p.position);
        
        // Calculate 3D distance like Java: getDistance(playerPos.addVector(0, (1.62F / 2), 0), skullPos)
        let player_center = DVec3::new(
            player_pos.x,
            player_pos.y + 0.81, // 1.62F / 2
            player_pos.z,
        );
        let distance_squared = entity.position.distance_to(&player_center).powi(2);
        
        // Check if player is within range
        if distance_squared <= JERRY_RANGE * JERRY_RANGE {
            // Calculate knockback direction like Java (3D, not just horizontal)
            let knockback_dir = DVec3::new(
                entity.position.x - player_pos.x,
                entity.position.y - (player_pos.y + 0.81), // 1.62F / 2
                entity.position.z - player_pos.z,
            ).normalize();
            
            let horizontal = JERRY_HORIZONTAL_MULT; // jerryVelo / 2
            
            entity.world_mut().server_mut().schedule(1, move |server| {
                if let Some(player) = server.world.players.get_mut(&client_id) {
                    // Apply knockback like Java: motionX += -knockbackDir.xCoord * horizontal
                    // Since we can't track velocity in Player, we'll just apply the knockback directly
                    let knockback_x = -knockback_dir.x * horizontal;
                    let knockback_y = JERRY_VELO; // Set Y motion directly like Java
                    let knockback_z = -knockback_dir.z * horizontal;
                    
                    player.write_packet(&EntityVelocity {
                        entity_id: VarInt(player.entity_id),
                        velocity_x: (knockback_x * 8000.0) as i16,
                        velocity_y: (knockback_y * 8000.0) as i16,
                        velocity_z: (knockback_z * 8000.0) as i16,
                    });
                }
            });
        }
    }
}

impl EntityImpl for JerryProjectileImpl {
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Initialize entity like EntityWitherSkull (no rotation)
        entity.pitch = 0.0;
        entity.yaw = 0.0; // No rotation like Java version
        
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
        // Check lifetime like Java: if (this.ticksExisted > 20 * 3 && !this.setDead)
        if entity.ticks_existed > MAX_LIFETIME_TICKS && !self.set_dead {
            self.set_dead = true;
            let entity_id = entity.id;
            entity.world_mut().server_mut().schedule(0, move |server| {
                server.world.despawn_entity(entity_id);
            });
            return;
        }

        let pre_pos = entity.position; // Store previous position BEFORE movement
        entity.position = DVec3::new(
            entity.position.x + self.velocity_per_tick.x,
            entity.position.y + self.velocity_per_tick.y,
            entity.position.z + self.velocity_per_tick.z,
        );
        let post_pos = entity.position; // Store post position

        // No rotation like Java version (unlike Bonzo)
        let yaw_byte = (entity.yaw * 256.0 / 360.0) as i8;
        let pitch_byte = (entity.pitch * 256.0 / 360.0) as i8;

        let delta_x = ((post_pos.x - pre_pos.x) * 32.0) as i8;
        let delta_y = ((post_pos.y - pre_pos.y) * 32.0) as i8;
        let delta_z = ((post_pos.z - pre_pos.z) * 32.0) as i8;

        for _player in entity.world_mut().players.values() {
            let _ = packet_buffer.write_packet(&EntityMoveRotate {
                entity_id: VarInt(entity.id),
                pos_x: delta_x,
                pos_y: delta_y,
                pos_z: delta_z,
                yaw: yaw_byte,
                pitch: pitch_byte,
                on_ground: entity.on_ground,
            });
        }

        if self.check_collision(entity) {
            self.schedule_knockback_for_next_tick(entity); // Scheduled knockback
            entity.world_mut().despawn_entity(entity.id);
            return;
        }
    }
    
    fn despawn(&mut self, _entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
        // Clean up if needed
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

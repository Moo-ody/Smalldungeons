use crate::net::protocol::play::clientbound::{EntityVelocity, EntityMoveRotate};
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::var_int::VarInt;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::player::player::ClientId;
use crate::server::utils::dvec3::DVec3;

// Core constants matching Java implementation
const TPS: f64 = 20.0; // Minecraft runs at 20 ticks per second
const PROJECTILE_SPEED: f64 = 20.0; // blocks per second
const MAX_LIFETIME_TICKS: u32 = 200; // 10 seconds (20 * 10)
const ROTATION_PER_TICK: f32 = 60.0; // Yaw increases by 60 each tick like Java
const BONZO_RANGE: f64 = 3.0; // Range for knockback (from DungeonSim.config.bonzoRange = 3F)
const BONZO_HORIZONTAL_MULT: f64 = 1.5; // Horizontal knockback multiplier (from DungeonSim.config.bonzoHMult = 1.5F)
const BONZO_VERTICAL_VELO: f64 = 0.5; // Vertical velocity for knockback (from DungeonSim.config.bonzoVVelo = 0.5F)

/// Bonzo projectile implementation matching Java version
pub struct BonzoProjectileImpl {
thrower_client_id: ClientId,
velocity_per_tick: DVec3,
current_yaw: f32, // For rotation like Java version
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
current_yaw: 0.0, // Start with 0 yaw
}
}

/// Set velocity after creation (for delayed launch system)
   pub fn set_velocity(&mut self, direction: DVec3, speed_bps: f64) {
self.velocity_per_tick = DVec3::new(
direction.x * (speed_bps / TPS),
direction.y * (speed_bps / TPS),
direction.z * (speed_bps / TPS),
);
}
}

impl EntityImpl for BonzoProjectileImpl {
fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
// Initialize entity like EntityWitherSkull
entity.pitch = 0.0; // Projectiles typically have 0 pitch
entity.yaw = self.current_yaw; // Set initial yaw

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
// Check lifetime limit (10 seconds = 200 ticks like Java)
if entity.ticks_existed > MAX_LIFETIME_TICKS {
entity.world_mut().despawn_entity(entity.id);
return;
}

// Store previous position BEFORE movement (like Java's prePos)
let pre_pos = entity.position;

// Move entity forward (like Java's moveEntity)
entity.position = DVec3::new(
entity.position.x + self.velocity_per_tick.x,
entity.position.y + self.velocity_per_tick.y,
entity.position.z + self.velocity_per_tick.z,
);

// Store post position (like Java's postPos)
let post_pos = entity.position;

// Calculate delta values exactly like Java version
let delta_x = ((post_pos.x - pre_pos.x) * 32.0) as i8;
let delta_y = ((post_pos.y - pre_pos.y) * 32.0) as i8;
let delta_z = ((post_pos.z - pre_pos.z) * 32.0) as i8;

// Update rotation INSIDE packet calculation like Java
// Java: byte yaw = (byte) ((this.rotationYaw += 60) * 256 / 360);
self.current_yaw += ROTATION_PER_TICK;
entity.yaw = self.current_yaw;
let yaw_byte = (self.current_yaw * 256.0 / 360.0) as i8;

// Java: byte pitch = (byte) (this.rotationPitch * 256 / 360);
// Use entity.pitch (which should be 0 for projectiles)
let pitch_byte = (entity.pitch * 256.0 / 360.0) as i8;

// Send packet to all players (like Java's Utils.sendPacketAll)
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

// Check for collision using bounding box (like Java's collision detection)
if self.check_collision(entity) {
// Schedule knockback for next tick (like Java's Utils.onWorldTick(0, ...))
self.schedule_knockback_for_next_tick(entity);
entity.world_mut().despawn_entity(entity.id);
return;
}
}
}

impl BonzoProjectileImpl {
/// Check for collision using bounding box (like Java's getCollidingBoundingBoxes)
   fn check_collision(&self, entity: &Entity) -> bool {
let world = entity.world_mut();

// Create bounding box like Java's getEntityBoundingBox()
// Java EntityWitherSkull has a bounding box of approximately 0.3125 x 0.3125 x 0.3125
let bb_size = 0.3125;
let half_size = bb_size / 2.0;

let bb_min_x = entity.position.x - half_size;
let bb_min_y = entity.position.y - half_size;
let bb_min_z = entity.position.z - half_size;
let bb_max_x = entity.position.x + half_size;
let bb_max_y = entity.position.y + half_size;
let bb_max_z = entity.position.z + half_size;

// Check collision with blocks in the bounding box (like Java's getCollidingBoundingBoxes)
let min_block_x = bb_min_x.floor() as i32;
let min_block_y = bb_min_y.floor() as i32;
let min_block_z = bb_min_z.floor() as i32;
let max_block_x = bb_max_x.ceil() as i32;
let max_block_y = bb_max_y.ceil() as i32;
let max_block_z = bb_max_z.ceil() as i32;

for x in min_block_x..=max_block_x {
for y in min_block_y..=max_block_y {
for z in min_block_z..=max_block_z {
let block = world.get_block_at(x, y, z);
if !is_block_passable_for_projectile(block) {
// Check if the block's bounding box intersects with projectile's bounding box
if self.bounding_box_intersects(
bb_min_x, bb_min_y, bb_min_z, bb_max_x, bb_max_y, bb_max_z,
x as f64, y as f64, z as f64, (x + 1) as f64, (y + 1) as f64, (z + 1) as f64
) {
return true;
}
}
}
}
}

false
}

/// Check if two bounding boxes intersect
   fn bounding_box_intersects(&self, 
min1_x: f64, min1_y: f64, min1_z: f64, max1_x: f64, max1_y: f64, max1_z: f64,
min2_x: f64, min2_y: f64, min2_z: f64, max2_x: f64, max2_y: f64, max2_z: f64
) -> bool {
min1_x < max2_x && max1_x > min2_x &&
min1_y < max2_y && max1_y > min2_y &&
min1_z < max2_z && max1_z > min2_z
}

/// Schedule knockback for next tick (like Java's Utils.onWorldTick(0, ...))
   fn schedule_knockback_for_next_tick(&self, entity: &Entity) {
let world = entity.world_mut();
let thrower_id = self.thrower_client_id;
let projectile_pos = entity.position;

// Find the shooting player
if let Some(shooter) = world.players.get(&thrower_id) {
let player_pos = shooter.position;

// Calculate distance from projectile to player center (like Java)
let player_center = DVec3::new(
player_pos.x,
player_pos.y + 0.9, // Player center height (1.8F / 2f in Java)
player_pos.z,
);

let distance_squared = projectile_pos.distance_to(&player_center).powi(2);

// Check if player is within range
if distance_squared <= BONZO_RANGE * BONZO_RANGE {
// Calculate knockback direction (from player to projectile, like Java)
let knockback_dir = DVec3::new(
projectile_pos.x - player_pos.x,
0.0, // Horizontal only like Java
projectile_pos.z - player_pos.z,
).normalize();

// Store knockback values for next tick
let knockback_x = -knockback_dir.x * BONZO_HORIZONTAL_MULT;
let knockback_y = BONZO_VERTICAL_VELO;
let knockback_z = -knockback_dir.z * BONZO_HORIZONTAL_MULT;

// Schedule knockback with 60ms delay (1.2 ticks ≈ 1 tick at 20 TPS)
let server = world.server_mut();
server.schedule(1, move |server| {
if let Some(shooter) = server.world.players.get_mut(&thrower_id) {
// Convert to velocity packet format (multiply by 8000)
let velocity_x = (knockback_x * 8000.0) as i16;
let velocity_y = (knockback_y * 8000.0) as i16;
let velocity_z = (knockback_z * 8000.0) as i16;

// Send velocity packet to player (like Java's velocityChanged = true)
shooter.write_packet(&EntityVelocity {
entity_id: VarInt(shooter.entity_id),
velocity_x,
velocity_y,
velocity_z,
});

}
});
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
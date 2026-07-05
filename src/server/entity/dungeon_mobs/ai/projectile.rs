//! Shared mob-fired projectiles: a straight-dropping arrow (Skeleton Soldier/Master/Lord,
//! Super Archer) and a straight-flying, exploding wither skull (Crypt Souleater's ranged
//! attack). Both go through the same generic `MobProjectileImpl`, parameterized by whether
//! gravity/drag and an impact explosion apply, rather than becoming separate bespoke
//! projectile files like the existing player-item projectiles each are.
//!
//! Position is driven by direct integration + `Entity::tick`'s existing automatic
//! teleport-on-move broadcast, deliberately simpler than the delta-packet dance in
//! `bonzo_projectile.rs` - fine for these, not trying to be a physically precise replica.

use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{Particles, SoundEffect};
use crate::server::block::block_collision::is_block_passable;
use crate::server::entity::dungeon_mobs::ai::movement::{pitch_towards, yaw_towards};
use crate::server::entity::entity::{Entity, EntityId, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;

const TICKS_PER_SECOND: f64 = 20.0;
const MAX_LIFETIME_TICKS: u32 = 100; // 5 seconds
/// Hit radius against a player - generous on purpose since there's no real hitbox/damage
/// system for this yet; the projectile just needs to visibly reach and despawn.
const HIT_RADIUS: f64 = 1.0;
/// Matches vanilla `EntityArrow`'s gravity/drag constants - keeping our server-authoritative
/// trajectory close to what the client independently predicts from the initial velocity
/// avoids visible "correction" jitter between the two. Wither skulls use neither (vanilla
/// `EntityWitherSkull` flies in a constant straight line).
const GRAVITY_PER_TICK: f64 = 0.05;
const DRAG_PER_TICK: f64 = 0.99;

struct MobProjectileImpl {
    velocity_per_tick: DVec3,
    gravity: bool,
    /// Wither skulls explode (particles + sound) on hitting a block or player; arrows just
    /// despawn quietly.
    explodes_on_impact: bool,
}

impl EntityImpl for MobProjectileImpl {
    fn spawn(&mut self, entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
        face_velocity(entity, self.velocity_per_tick);
    }

    fn tick(&mut self, entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
        if entity.ticks_existed > MAX_LIFETIME_TICKS {
            entity.world_mut().despawn_entity(entity.id);
            return;
        }

        entity.position = entity.position + self.velocity_per_tick;
        face_velocity(entity, self.velocity_per_tick);

        if self.gravity {
            // Vanilla arrow physics: gravity pulls it down, drag slows it, applied once per
            // tick after moving - matches EntityArrow's own onUpdate order closely enough
            // that our periodic position corrections shouldn't visibly fight the client's
            // own prediction.
            self.velocity_per_tick.y -= GRAVITY_PER_TICK;
            self.velocity_per_tick.x *= DRAG_PER_TICK;
            self.velocity_per_tick.y *= DRAG_PER_TICK;
            self.velocity_per_tick.z *= DRAG_PER_TICK;
        }

        let world = entity.world_mut();
        let block = world.get_block_at(
            entity.position.x.floor() as i32,
            entity.position.y.floor() as i32,
            entity.position.z.floor() as i32,
        );
        if !is_block_passable(block) {
            if self.explodes_on_impact {
                explode(world, entity.position);
            }
            world.despawn_entity(entity.id);
            return;
        }

        // No mob-vs-player damage system exists yet - despawning on proximity is the
        // placeholder "the shot landed" signal (see ai/attack.rs TODO).
        let hit_player = world.players.values().any(|player| player.position.distance_to(&entity.position) < HIT_RADIUS);
        if hit_player {
            if self.explodes_on_impact {
                explode(world, entity.position);
            }
            world.despawn_entity(entity.id);
        }
    }
}

/// Visual + sound explosion effect at `pos` (no real AOE damage - there's no player damage
/// system anywhere in this codebase to plug into yet).
fn explode(world: &mut World, pos: DVec3) {
    let particles = Particles {
        particle_id: 1, // largeexplode
        long_distance: true,
        x: pos.x as f32,
        y: pos.y as f32,
        z: pos.z as f32,
        offset_x: 0.0,
        offset_y: 0.0,
        offset_z: 0.0,
        speed: 0.0,
        count: 0,
    };
    for player in world.players.values_mut() {
        player.write_packet(&particles);
        player.write_packet(&SoundEffect {
            sound: Sounds::RandomExplode.id(),
            pos_x: pos.x,
            pos_y: pos.y,
            pos_z: pos.z,
            volume: 1.0,
            pitch: 1.0,
        });
    }
}

/// Points the projectile along its current velocity vector, matching vanilla's own
/// velocity-follows-rotation behavior (otherwise it visibly flies sideways/upside-down
/// relative to its actual movement).
fn face_velocity(entity: &mut Entity, velocity: DVec3) {
    let horizontal = (velocity.x * velocity.x + velocity.z * velocity.z).sqrt();
    entity.yaw = yaw_towards(velocity.x, velocity.z);
    entity.pitch = pitch_towards(velocity.y, horizontal);
}

fn fire_projectile(world: &mut World, from: DVec3, target_pos: DVec3, speed_bps: f64, variant: EntityVariant, gravity: bool, explodes_on_impact: bool) -> anyhow::Result<EntityId> {
    let direction = (target_pos - from).normalize();
    let velocity_per_tick = DVec3::new(
        direction.x * speed_bps / TICKS_PER_SECOND,
        direction.y * speed_bps / TICKS_PER_SECOND,
        direction.z * speed_bps / TICKS_PER_SECOND,
    );

    // `spawn_entity_with_velocity` sets `entity.velocity` before the SpawnObject packet is
    // written, which is what lets the client render/predict the projectile's motion
    // correctly from frame one (see `EntityVariant::object_data` - SpawnObject only sends
    // velocity when `data > 0`).
    let metadata = EntityMetadata::new(variant);
    world.spawn_entity_with_velocity(from, velocity_per_tick, metadata, MobProjectileImpl { velocity_per_tick, gravity, explodes_on_impact })
}

/// Fires a straight-line, gravity-affected arrow from `from` toward `target_pos` (the
/// target's position at the moment of firing - vanilla arrows don't home in on a moving
/// target either).
pub fn fire_arrow_at_player(world: &mut World, from: DVec3, target_pos: DVec3, speed_bps: f64) -> anyhow::Result<EntityId> {
    fire_projectile(world, from, target_pos, speed_bps, EntityVariant::Arrow, true, false)
}

/// Fires a straight-line, non-dropping wither skull from `from` toward `target_pos` (Crypt
/// Souleater's ranged attack) that explodes on hitting a block or player.
pub fn fire_wither_skull_at_player(world: &mut World, from: DVec3, target_pos: DVec3, speed_bps: f64) -> anyhow::Result<EntityId> {
    fire_projectile(world, from, target_pos, speed_bps, EntityVariant::WitherSkullProjectile, false, true)
}

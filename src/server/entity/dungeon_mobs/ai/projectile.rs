//! Shared mob-fired projectiles: a straight-dropping arrow (Skeleton Soldier/Master/Lord,
//! Super Archer), a straight-flying exploding wither skull (Crypt Souleater's ranged attack),
//! and a straight-flying fishing hook that, on touching a player, purely cosmetically flies back
//! to the shooter instead of despawning outright (Zombie Commander - "cast rod at me and reel,
//! but it shouldn't do anything" - no pull, no damage). All three go through the same generic
//! `MobProjectileImpl`, parameterized by an `ImpactEffect` (despawn / explode / reel in) rather
//! than becoming separate bespoke projectile files like the existing player-item projectiles
//! each are.
//!
//! Position is driven by direct integration, but broadcast via our own manual relative-move
//! packet each tick (see `tick()`'s end) rather than `Entity::tick`'s generic automatic
//! broadcast, which only ever sends an *absolute* teleport - fine for slow/occasional movers,
//! but a hard absolute snap every single tick for a fast-moving projectile reads as jittery/
//! not-smooth instead of a continuous line (confirmed by comparison: `bonzo_projectile.rs`/
//! `jerry_projectile.rs` already send small relative-move deltas every tick for exactly this
//! reason, and don't have this problem - this now does the same instead of the simpler-but-
//! choppier generic path it originally used).

use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{EntityMoveRotate, EntityTeleport, Particles, SoundEffect};
use crate::net::var_int::VarInt;
use crate::server::block::block_collision::is_block_passable;
use crate::server::block::block_position::BlockPos;
use crate::server::entity::entity::{Entity, EntityId, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::ClientId;
use crate::server::utils::aabb::AABB;
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

/// What happens when a projectile actually reaches a block or player.
#[derive(Clone, Copy)]
pub(crate) enum ImpactEffect {
    /// Arrows: despawn quietly, no visual effect.
    Despawn,
    /// Wither skulls: particles + explosion sound, then despawn.
    Explode,
    /// Zombie Commander's fishing hook: "cast rod at me and reel, but it shouldn't do anything" -
    /// purely cosmetic. On touching a player the hook switches to `ReelState` and visually
    /// flies back to `shooter_id`'s live position over `REEL_TICKS`, then despawns; no effect on
    /// the player whatsoever (no pull, no damage). Only triggers on hitting a *player* - hitting
    /// a block first just despawns it like an arrow (a rod cast into a wall doesn't hook anyone).
    ReelIn { shooter_id: EntityId },
}

/// How long a hooked player gets reeled in for before the hook gives up and despawns - long
/// enough to cross a typical dungeon room. ~1 second at 20 TPS.
const REEL_TICKS: u32 = 20;

struct ReelState {
    shooter_id: EntityId,
    ticks_left: u32,
}

struct MobProjectileImpl {
    velocity_per_tick: DVec3,
    gravity: bool,
    impact: ImpactEffect,
    /// `Some` once `impact` is `ReelIn` and the hook has actually connected - while active, the
    /// hook stops flying/despawns-on-block-hit and instead ticks its (purely visual) retract
    /// back toward the shooter.
    reeling: Option<ReelState>,
    /// `Some(shooter)` lets this projectile trigger `world.interactable_blocks` (puzzle chests/
    /// levers/the Creeper Beams lanterns) on a block hit, treating the shot as if `shooter` had
    /// clicked that block directly - see `fire_projectile`'s doc comment. Deliberately `None`
    /// for every mob-fired projectile (`fire_arrow_at_player` etc.): a stray skeleton arrow
    /// shouldn't be able to open a chest or complete a puzzle pair, only a player's own real
    /// shot (currently only the Terminator, see `items::terminator`) should.
    on_block_hit: Option<ClientId>,
}

impl EntityImpl for MobProjectileImpl {
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        face_velocity(entity, self.velocity_per_tick);

        // For a non-player entity, `write_entity_spawn` writes the `SpawnObject`/`SpawnMob`
        // packet *before* calling this - so that packet went out with the entity's default
        // yaw/pitch (0/0, dead south/level), not the `face_velocity` orientation just set above.
        // Left alone, the client renders it facing south for one tick and then snaps to the
        // real direction the instant the first `tick()` broadcast arrives - a visible "spin" on
        // every single arrow, worse with 3 at once (Terminator's multi-shot). Writing an
        // immediate follow-up `EntityTeleport` into this same spawn packet batch corrects it
        // before the client ever renders a frame with the wrong orientation, instead of waiting
        // a tick for the fix to arrive as a visibly separate snap.
        packet_buffer.write_packet(&crate::net::protocol::play::clientbound::EntityTeleport {
            entity_id: entity.id,
            pos_x: entity.position.x,
            pos_y: entity.position.y,
            pos_z: entity.position.z,
            yaw: entity.yaw,
            pitch: entity.pitch,
            on_ground: false,
        });
    }

    fn tick(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        if let Some(reel) = &mut self.reeling {
            tick_reel(entity, reel);
            return;
        }

        if entity.ticks_existed > MAX_LIFETIME_TICKS {
            entity.world_mut().despawn_entity(entity.id);
            return;
        }

        let pre_pos = entity.position;

        // Vanilla `EntityArrow.h()` (`onUpdate`)'s real order, verified against Mojang's actual
        // decompiled server source (the historical `Bukkit/mc-dev` reference): move first using
        // last tick's velocity, *then* drag, *then* gravity - gravity's effect on a given tick's
        // move only shows up the *following* tick. (An earlier attempt "fixed" this backwards
        // based on a bad memory of the order - this is the real, source-verified one.)
        entity.position = entity.position + self.velocity_per_tick;

        // Deliberately *not* re-orienting to the new (gravity-curved) velocity every tick
        // anymore, unlike a real vanilla arrow - `face_velocity` still runs once, in `spawn()`,
        // so it launches pointing exactly along its initial path and simply keeps that fixed
        // orientation for its whole flight instead of continuously re-aiming itself. Per
        // explicit request: this stops the tip from turning at all, and removes rotation as a
        // moving part in whatever was still causing the remaining glitchiness.
        if self.gravity {
            self.velocity_per_tick.x *= DRAG_PER_TICK;
            self.velocity_per_tick.y *= DRAG_PER_TICK;
            self.velocity_per_tick.z *= DRAG_PER_TICK;
            self.velocity_per_tick.y -= GRAVITY_PER_TICK;
        }

        // Small relative-move-and-look delta, not an absolute teleport - see the module doc
        // comment - *unless* this tick's actual move is too big for one to represent: its
        // fields are signed bytes in 1/32-block units, ±127 -> a hard ±3.97 block/tick range.
        // Gravity keeps accumulating on a falling arrow's downward speed every tick (this
        // codebase's drag/gravity constants have it approach a ~5 block/tick terminal velocity),
        // so a long-enough flight eventually *exceeds* that range - and `as i8` on an
        // out-of-range float saturates instead of erroring, so the delta silently clamps short
        // of the real drop instead of failing loudly. That's a growing, compounding desync
        // between the client's rendered position and the server's real one every tick after -
        // exactly "the further it flies, the worse it spazzes out". Real vanilla's own entity
        // tracker falls back to an absolute teleport whenever a delta would be this large,
        // instead of ever letting one clamp - matched here the same way.
        let delta = DVec3::new(entity.position.x - pre_pos.x, entity.position.y - pre_pos.y, entity.position.z - pre_pos.z);
        let yaw_byte = (entity.yaw * 256.0 / 360.0) as i32 as i8;
        let pitch_byte = (entity.pitch * 256.0 / 360.0) as i32 as i8;
        if delta.x.abs() >= 4.0 || delta.y.abs() >= 4.0 || delta.z.abs() >= 4.0 {
            packet_buffer.write_packet(&EntityTeleport {
                entity_id: entity.id,
                pos_x: entity.position.x,
                pos_y: entity.position.y,
                pos_z: entity.position.z,
                yaw: entity.yaw,
                pitch: entity.pitch,
                on_ground: entity.on_ground,
            });
        } else {
            packet_buffer.write_packet(&EntityMoveRotate {
                entity_id: VarInt(entity.id),
                pos_x: (delta.x * 32.0) as i8,
                pos_y: (delta.y * 32.0) as i8,
                pos_z: (delta.z * 32.0) as i8,
                yaw: yaw_byte,
                pitch: pitch_byte,
                on_ground: entity.on_ground,
            });
        }
        // `Entity::tick`'s own generic "position changed since last tick" check runs right
        // after this function returns - stamping this now means it finds nothing to do and
        // doesn't *also* send its own redundant absolute teleport for the same tick.
        entity.last_position = entity.position;

        let world = entity.world_mut();
        // Entities are only swept for a player-fired shot (Terminator, via `on_block_hit`) - same
        // "only a deliberate player shot, never a mob-fired one" gating `on_block_hit`'s own doc
        // comment already establishes for block hits, reused here for entity hits (a stray
        // skeleton arrow shouldn't be able to redirect the Ice Path silverfish either).
        let check_entities = self.on_block_hit.is_some();
        // Re-swept in a loop (not a single call) to support piercing - a hit whose own
        // `on_projectile_hit` returns `false` (see that method's own doc comment) doesn't stop
        // the projectile, so the *same* tick's flight can go on to hit more entities behind the
        // first one (the Higher or Lower puzzle's blazes, per explicit request) before it's
        // finally blocked or reaches its endpoint.
        let mut pierced: Vec<EntityId> = Vec::new();
        loop {
            match sweep_hit(world, pre_pos, entity.position, entity.id, &pierced, check_entities) {
                Some(SweepHit::Entity(hit_id)) => {
                    // Guaranteed `Some` - `check_entities` (and therefore any `SweepHit::Entity`
                    // result at all) only happens when `on_block_hit` is `Some`.
                    let shooter_id = self.on_block_hit.unwrap();
                    let velocity = self.velocity_per_tick;
                    let stop = world.entities.get_mut(&hit_id)
                        .map(|(other_entity, other_impl)| other_impl.on_projectile_hit(other_entity, velocity, shooter_id))
                        .unwrap_or(true);
                    if stop {
                        world.despawn_entity(entity.id);
                        return;
                    }
                    pierced.push(hit_id);
                }
                Some(SweepHit::Block(hit_pos)) => {
                    if matches!(self.impact, ImpactEffect::Explode) {
                        explode(world, entity.position);
                    }
                    // Disjoint-field borrows of `world.interactable_blocks` and `world.players` at
                    // once - same aliasing this codebase's whole block-interact dispatch already
                    // relies on (every `BlockInteractAction::interact` implementation freely reaches
                    // back into `World`/`Dungeon` via the shooter `Player`'s own
                    // `server_mut()`/`world_mut()` while this `action` borrow is nominally still
                    // live, e.g. `RedstoneKeySkull`'s handler removing itself from this very map) -
                    // not introducing a new pattern, just reusing the existing one from a projectile
                    // hit instead of a direct player click.
                    // `on_block_hit` is only ever `Some` for a Terminator arrow (`terminator.rs`
                    // is the sole caller that passes it) - per explicit correction, the Terminator
                    // is a Creeper Beams-specific bow trick shot, so this must never open/trigger
                    // any OTHER puzzle's interactable (secret chests, essence, levers, doors,
                    // etc.) just because an arrow happened to land on one, only that one lantern.
                    if let Some(shooter_id) = self.on_block_hit {
                        let is_creeper_beams_lantern = matches!(
                            world.interactable_blocks.get(&hit_pos),
                            Some(crate::server::block::block_interact_action::BlockInteractAction::CreeperBeamsLantern { .. })
                        );
                        if is_creeper_beams_lantern {
                            if let (Some(action), Some(shooter)) = (world.interactable_blocks.get(&hit_pos), world.players.get_mut(&shooter_id)) {
                                action.interact(shooter, &hit_pos);
                            }
                        }
                    }
                    world.despawn_entity(entity.id);
                    return;
                }
                None => break,
            }
        }

        // No mob-vs-player damage system exists yet - despawning on proximity is the
        // placeholder "the shot landed" signal (see ai/attack.rs TODO). `ReelIn` is the one
        // exception that doesn't despawn immediately on contact - it switches to the (purely
        // cosmetic) retract animation below instead.
        let hit_player = world.players.values().any(|player| player.position.distance_to(&entity.position) < HIT_RADIUS);
        if !hit_player {
            return;
        }

        match self.impact {
            ImpactEffect::Despawn => world.despawn_entity(entity.id),
            ImpactEffect::Explode => {
                explode(world, entity.position);
                world.despawn_entity(entity.id);
            }
            ImpactEffect::ReelIn { shooter_id } => {
                self.reeling = Some(ReelState { shooter_id, ticks_left: REEL_TICKS });
            }
        }
    }
}

/// One tick of the hook visually flying back to the shooter after hitting a player - cosmetic
/// only, no effect on the player whatsoever (no pull, no damage - "it shouldn't do anything").
/// Ends (and despawns the hook) once `ticks_left` runs out or the shooter is gone (despawned
/// mid-reel).
fn tick_reel(entity: &mut Entity, reel: &mut ReelState) {
    let world = entity.world_mut();

    let Some((shooter_entity, _)) = world.entities.get(&reel.shooter_id) else {
        world.despawn_entity(entity.id);
        return;
    };
    let shooter_pos = shooter_entity.position;

    if reel.ticks_left == 0 || entity.position.distance_to(&shooter_pos) < HIT_RADIUS {
        world.despawn_entity(entity.id);
        return;
    }
    reel.ticks_left -= 1;

    let remaining = shooter_pos - entity.position;
    let divisor = reel.ticks_left as f64 + 1.0;
    let velocity = DVec3::new(remaining.x / divisor, remaining.y / divisor, remaining.z / divisor);
    entity.position = entity.position + velocity;
    face_velocity(entity, velocity);
}

/// What `sweep_hit` found first along a tick's movement segment.
enum SweepHit {
    Block(BlockPos),
    Entity(EntityId),
}

/// Real vanilla (width, height) collision box for a `wants_projectile_hits`-opted-in entity
/// variant - only variants that actually opt in need an entry; anything else falls back to a
/// generic mob-sized box rather than panicking, since a future opt-in shouldn't have to remember
/// to update this too.
fn hitbox_for(variant: &EntityVariant) -> (f64, f64) {
    match variant {
        EntityVariant::Blaze => (0.6, 1.8),
        EntityVariant::Silverfish => (0.3, 0.7),
        _ => (0.6, 1.95),
    }
}

/// Real vanilla `EntityArrow` collision margin - straight from the decompiled 1.8 server source,
/// which tests a candidate's `getEntityBoundingBox().expand(0.3, 0.3, 0.3)`, not its raw hitbox.
/// Without this, a shot that clips just past a target's real edge (a graze real vanilla still
/// registers as a hit) misses here instead - "the arrow hits the edge but doesn't count".
const ARROW_COLLISION_EXPAND: f64 = 0.3;

/// Walks the straight segment `from -> to` in small steps, returning whichever of (optionally) a
/// live entity or a non-passable block the projectile reaches *first* along it, if either.
/// Checking only a tick's *endpoint* position (the old approach) lets a projectile moving several
/// blocks in one tick - the Terminator's arrows travel 3 blocks/tick at real vanilla arrow speed -
/// tunnel straight through a single-block-thick target: a sea lantern's whole 1-block
/// cross-section (or, for `check_entities`, the Ice Path silverfish's whole hit radius) can fall
/// entirely between two consecutive endpoint samples, with more ticks (and more chances to get
/// skipped over on some tick) the farther away the target is - matching "can't shoot lanterns
/// from far, registers weirdly" (blocks) or "shooting the silverfish doesn't do anything" (the
/// same bug, just for entities - which had no sweep at all before, only a same-tick endpoint
/// check). Step size is well under 1 block so the segment can't skip over anything it actually
/// passes through. Entities aren't checked at all unless `check_entities` is set (only a
/// deliberate player shot should be able to redirect one - see the call site's doc comment) - a
/// skipped scan there is a plain early `continue`, not a correctness issue, since a `false`
/// `check_entities` caller never wants an `Entity` result regardless.
///
/// `already_hit` additionally excludes every entity this same tick's flight has already pierced
/// through (see `EntityImpl::on_projectile_hit`'s own doc comment on piercing) - re-sweeping the
/// same full remaining segment after a pierced hit would otherwise just find that same entity
/// again and never progress.
///
/// Entity hits are tested against each candidate's own real hitbox (`hitbox_for`), expanded by
/// `ARROW_COLLISION_EXPAND` exactly like real vanilla's `EntityArrow` does - not a flat
/// `HIT_RADIUS`-sphere around its feet position. A uniform 1-block-radius sphere anchored at feet
/// height was both too loose horizontally (a Higher or Lower shot at the correct next Blaze could
/// also graze an unrelated, out-of-order Blaze standing nearby along the same vertical shaft,
/// instantly failing a puzzle the player thought they'd just gotten right) and too tight
/// vertically for anyone aiming at center-mass instead of the very base of a tall mob (a real
/// Blaze is 1.8 blocks tall - aiming at its middle puts the arrow ~0.9 blocks above its
/// feet-anchored position, already most of the way to falling outside a radius-1.0 sphere, which
/// is exactly "doesn't hit most of the time"). The raw (unexpanded) hitbox alone still missed
/// genuine edge grazes real vanilla counts as a hit - `expand` closes that last gap.
fn sweep_hit(world: &World, from: DVec3, to: DVec3, exclude_id: EntityId, already_hit: &[EntityId], check_entities: bool) -> Option<SweepHit> {
    const STEP: f64 = 0.2;
    let delta = to - from;
    let dist = (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt();
    let steps = ((dist / STEP).ceil() as i32).max(1);

    let mut last_block_pos: Option<BlockPos> = None;
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let p = DVec3::new(from.x + delta.x * t, from.y + delta.y * t, from.z + delta.z * t);

        if check_entities {
            // Explicit opt-in (`EntityImpl::wants_projectile_hits`), not a heuristic - see that
            // method's doc comment for why a broader "any visible, non-dropped-item entity"
            // guess (tried first) was a real regression: it let a Terminator shot hit Creeper
            // Beams' visible Creeper prop instead of the sea lantern behind it.
            let hit = world.entities.iter()
                .find(|&(&id, (other, impl_))| {
                    if id == exclude_id || already_hit.contains(&id) || !impl_.wants_projectile_hits() {
                        return false;
                    }
                    let (width, height) = hitbox_for(&other.metadata.variant);
                    AABB::from_height_width(height, width).offset(other.position).expand(ARROW_COLLISION_EXPAND).contains(p)
                })
                .map(|(&id, _)| id);
            if let Some(id) = hit {
                return Some(SweepHit::Entity(id));
            }
        }

        let pos = BlockPos::new(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        if Some(pos) == last_block_pos {
            continue;
        }
        last_block_pos = Some(pos);
        if !is_block_passable(world.get_block_at(pos.x, pos.y, pos.z)) {
            return Some(SweepHit::Block(pos));
        }
    }
    None
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
    // Deliberately *not* `movement::yaw_towards`/`pitch_towards` - those implement a different,
    // separately-correct vanilla formula (mob body-facing, with its own -90 offset and sign
    // convention). Real vanilla `EntityArrow` uses its own plain, unnegated
    // `atan2(motionX, motionZ)` / `atan2(motionY, horizontalDist)` - verified directly against
    // Mojang's decompiled server source (`Bukkit/mc-dev`'s `EntityArrow.java`, both its `shoot()`
    // and per-tick `h()`). Reusing the mob formula here silently mirrored/flipped a projectile's
    // orientation relative to its real direction of travel on almost every non-cardinal angle -
    // constant and present from the very first frame, which is exactly what "looks wrong the
    // entire time" (not worsening with distance/time) was actually pointing at.
    entity.yaw = velocity.x.atan2(velocity.z).to_degrees() as f32;
    entity.pitch = velocity.y.atan2(horizontal).to_degrees() as f32;
}

/// `pub(crate)`, not just used by the `fire_*_at_player` wrappers below - `items::terminator`
/// also fires real arrows through this same physics/collision machinery (gravity, drag, block
/// and player hit detection) rather than duplicating a parallel arrow implementation for a
/// player-fired one; arrows behave identically regardless of who fired them, matching real
/// vanilla (one `EntityArrow` class for both).
pub(crate) fn fire_projectile(world: &mut World, from: DVec3, target_pos: DVec3, speed_bps: f64, variant: EntityVariant, gravity: bool, impact: ImpactEffect, on_block_hit: Option<ClientId>) -> anyhow::Result<EntityId> {
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
    world.spawn_entity_with_velocity(from, velocity_per_tick, metadata, MobProjectileImpl { velocity_per_tick, gravity, impact, reeling: None, on_block_hit })
}

/// Fires a straight-line, gravity-affected arrow from `from` toward `target_pos` (the
/// target's position at the moment of firing - vanilla arrows don't home in on a moving
/// target either).
pub fn fire_arrow_at_player(world: &mut World, from: DVec3, target_pos: DVec3, speed_bps: f64) -> anyhow::Result<EntityId> {
    fire_projectile(world, from, target_pos, speed_bps, EntityVariant::Arrow, true, ImpactEffect::Despawn, None)
}

/// Fires a straight-line, non-dropping wither skull from `from` toward `target_pos` (Crypt
/// Souleater's ranged attack) that explodes on hitting a block or player.
pub fn fire_wither_skull_at_player(world: &mut World, from: DVec3, target_pos: DVec3, speed_bps: f64) -> anyhow::Result<EntityId> {
    fire_projectile(world, from, target_pos, speed_bps, EntityVariant::WitherSkullProjectile, false, ImpactEffect::Explode, None)
}

/// Casts Zombie Commander's fishing hook from `from` toward `target_pos` - a straight-line,
/// non-dropping cast like the wither skull, but on actually touching a player it reels them in
/// (`ImpactEffect::ReelIn`) instead of exploding or despawning. `shooter_id` is the Zombie
/// Commander's own entity id, looked up again live each reel tick so the pull still tracks
/// correctly even if it's moved since casting.
pub fn fire_fishing_hook_at_player(world: &mut World, from: DVec3, target_pos: DVec3, speed_bps: f64, shooter_id: EntityId) -> anyhow::Result<EntityId> {
    fire_projectile(world, from, target_pos, speed_bps, EntityVariant::FishingHook, false, ImpactEffect::ReelIn { shooter_id }, None)
}

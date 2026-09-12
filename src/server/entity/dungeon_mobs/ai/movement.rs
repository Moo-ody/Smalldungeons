//! Movement styles + steering. No pathfinding exists anywhere in this codebase (the only
//! candidate, `server::old_entity`, is dead/uncompiled code) - movement here is direct-vector
//! steering (walk straight toward a point, turn to face it), with horizontal steps run
//! through `physics::move_horizontal` so mobs still can't walk through walls or players.
//! It can still get stuck circling an obstacle it can't step around; that's an accepted
//! placeholder limitation until real pathfinding exists.

use crate::server::entity::dungeon_mobs::ai::physics;
use crate::server::entity::entity::Entity;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use rand::Rng;

#[derive(Clone, Copy)]
pub enum MovementStyle {
    /// Walk straight toward the target until in melee range.
    Approach,
    /// Try to hold `preferred` blocks from the target (+/- `tolerance`) - approaches if
    /// farther than that, but never backs away just for being closer than preferred
    /// (matches the documented skeleton behavior: "does not necessarily retreat").
    MaintainDistance { preferred: f64, tolerance: f64 },
    /// Crypt Souleater's documented hybrid: closes fully within `melee_range`, otherwise
    /// holds around `ranged_preferred` (+/- `ranged_tolerance`) like `MaintainDistance`.
    /// Paired with `AttackModule::HybridMeleeRanged` using the same `melee_range`.
    HybridMeleeRanged { melee_range: f64, ranged_preferred: f64, ranged_tolerance: f64 },
    /// Crypt Dreadlord's documented melee behavior: closes in normally (same as `Approach`)
    /// until within `melee_range`, then orbits the target instead of standing still or
    /// continuing to push directly into them - `ai/mod.rs`'s attack-range face-lock (see there)
    /// keeps head/aim on the target the whole time regardless. Paired with `AttackModule::Melee`
    /// using the same `melee_range`. `orbit_radius` is the distance the orbit itself actively
    /// settles into/holds (correcting inward or outward as needed - e.g. backing off if it ends
    /// up standing right on the target) rather than just preserving whatever distance it
    /// happened to be at on crossing `melee_range` - usually the same value as `melee_range`
    /// (Crypt Dreadlord), but can be tighter (Lost/Frozen/Angry Adventurer's "up close" ~1 block
    /// hover, per explicit request).
    CircleStrafe { melee_range: f64, orbit_radius: f64 },
    /// Lost/Frozen/Angry Adventurer's documented melee behavior: closes in normally until
    /// within `melee_range`, then a slow side-to-side wiggle (not a full orbit like
    /// `CircleStrafe` - explicit request: "shouldn't try to strafe around me totally... strafing
    /// left and right a bit") toward `orbit_radius`, same radial-correction reasoning as
    /// `CircleStrafe`'s own `orbit_radius`. Paired with `AttackModule::HybridMeleeRanged` using
    /// the same `melee_range`.
    LightStrafe { melee_range: f64, orbit_radius: f64 },
    // Phase 3+: SprintJump { .. } - enum extends here, the pipeline in ai/mod.rs does not need
    // to change when it's added.
}

/// Ticks per second, matching the server's fixed 20 TPS main loop.
const TICKS_PER_SECOND: f64 = 20.0;

/// `target_pos` is used for every distance/hold decision (is the real target in range, too
/// close, too far); `steer_pos` is the point actually walked toward when approaching - the
/// next pathfinding waypoint (see `pathfinding::next_steer_point`), or `target_pos` itself
/// when no path is available. Kept separate so a maintain-distance archetype still measures
/// its distance against the real target even while walking toward an intermediate waypoint.
/// `allow_jump` should be `next_steer_point`'s own "is `steer_pos` a real resolved waypoint"
/// flag - see `physics::move_horizontal`'s doc comment for why jumping is gated on that.
pub fn apply_movement_style(
    entity: &mut Entity,
    world: &World,
    width: f64,
    height: f64,
    style: MovementStyle,
    target_pos: DVec3,
    steer_pos: DVec3,
    speed_bps: f64,
    allow_jump: bool,
) {
    match style {
        MovementStyle::Approach => steer_toward(entity, world, width, height, steer_pos, speed_bps, allow_jump),
        MovementStyle::MaintainDistance { preferred, tolerance } => {
            let distance = entity.position.distance_to(&target_pos);
            if distance > preferred + tolerance {
                steer_toward(entity, world, width, height, steer_pos, speed_bps, allow_jump);
            } else {
                face_toward(entity, target_pos);
            }
        }
        MovementStyle::HybridMeleeRanged { melee_range, ranged_preferred, ranged_tolerance } => {
            let distance = entity.position.distance_to(&target_pos);
            if distance < melee_range || distance > ranged_preferred + ranged_tolerance {
                steer_toward(entity, world, width, height, steer_pos, speed_bps, allow_jump);
            } else {
                // Holding at ranged distance to fire - "slightly strafe left and right while
                // firing wither skulls" rather than standing bolt still.
                strafe_lightly(entity, world, width, height, target_pos);
                face_toward(entity, target_pos);
            }
        }
        MovementStyle::CircleStrafe { melee_range, orbit_radius } => {
            let distance = entity.position.distance_to(&target_pos);
            if distance > melee_range {
                steer_toward(entity, world, width, height, steer_pos, speed_bps, allow_jump);
            } else {
                strafe_around_target(entity, world, width, height, target_pos, orbit_radius, speed_bps);
            }
        }
        MovementStyle::LightStrafe { melee_range, orbit_radius } => {
            let distance = entity.position.distance_to(&target_pos);
            if distance > melee_range {
                steer_toward(entity, world, width, height, steer_pos, speed_bps, allow_jump);
            } else {
                strafe_lightly_toward_radius(entity, world, width, height, target_pos, orbit_radius);
            }
        }
    }
}

/// How often (ticks) a circle-strafing mob flips its orbit direction - long enough to read as a
/// deliberate arc rather than a jittery back-and-forth, short enough it doesn't just look like
/// it's endlessly circling one way.
const STRAFE_FLIP_INTERVAL_TICKS: u32 = 40;

/// Orbits `target_pos`, walking tangentially instead of directly toward/away - used once a
/// `CircleStrafe` mob is already within `melee_range`. Direction alternates over time (see
/// `STRAFE_FLIP_INTERVAL_TICKS`) purely from `entity.ticks_existed`, so no extra per-mob state is
/// needed to track which way it's currently circling. Blended with a radial correction back
/// toward `orbit_radius` (positive = outward) rather than just preserving whatever distance it
/// happened to already be at - otherwise a mob that ends up standing right on top of its target
/// (`dx`/`dz` near zero) just orbits in place at that same degenerate radius forever instead of
/// actually backing off to a sensible fighting distance. Doesn't touch yaw/pitch - `ai/mod.rs`'s
/// attack-range face lock handles looking at the target on top of this.
fn strafe_around_target(entity: &mut Entity, world: &World, width: f64, height: f64, target_pos: DVec3, orbit_radius: f64, speed_bps: f64) {
    let dx = entity.position.x - target_pos.x;
    let dz = entity.position.z - target_pos.z;
    let radius = (dx * dx + dz * dz).sqrt().max(0.5);
    let direction = if (entity.ticks_existed / STRAFE_FLIP_INTERVAL_TICKS) % 2 == 0 { 1.0 } else { -1.0 };
    // Tangent to the target->mob radius, rotated 90 degrees.
    let (tangent_x, tangent_z) = (-dz / radius * direction, dx / radius * direction);
    let radial_bias = ((orbit_radius - radius) / orbit_radius.max(0.5)).clamp(-1.0, 1.0);
    let (radial_x, radial_z) = (dx / radius * radial_bias, dz / radius * radial_bias);

    let combined_x = tangent_x + radial_x;
    let combined_z = tangent_z + radial_z;
    let len = (combined_x * combined_x + combined_z * combined_z).sqrt().max(1e-6);

    let step = speed_bps / TICKS_PER_SECOND;
    physics::move_horizontal(entity, world, width, height, combined_x / len * step, combined_z / len * step, false);
}

/// Orbits `target_pos` like `strafe_around_target` above, but in a persisted direction
/// (`clockwise`) instead of `strafe_around_target`'s time-based flip, blended with an
/// inward/outward radial component (`radial_bias`: negative darts toward the target, positive
/// retreats away from it, `0.0` is a pure orbit) - Shadow Assassin's documented "spins one
/// direction, weaves in to strike, weaves back out" pattern (see `ai/mod.rs`'s `teleport_ambush`
/// branch, which owns the persisted direction/weave-phase state this needs). Doesn't touch
/// yaw/pitch, same as `strafe_around_target` - the caller's own face-lock handles that.
pub fn orbit_and_weave(entity: &mut Entity, world: &World, width: f64, height: f64, target_pos: DVec3, clockwise: bool, radial_bias: f64, speed_bps: f64) {
    let dx = entity.position.x - target_pos.x;
    let dz = entity.position.z - target_pos.z;
    let radius = (dx * dx + dz * dz).sqrt().max(0.5);
    let direction = if clockwise { 1.0 } else { -1.0 };
    let (tangent_x, tangent_z) = (-dz / radius * direction, dx / radius * direction);
    let (radial_x, radial_z) = (dx / radius * radial_bias, dz / radius * radial_bias);

    let combined_x = tangent_x + radial_x;
    let combined_z = tangent_z + radial_z;
    let len = (combined_x * combined_x + combined_z * combined_z).sqrt();
    if len < 1e-6 {
        return;
    }

    let step = speed_bps / TICKS_PER_SECOND;
    physics::move_horizontal(entity, world, width, height, combined_x / len * step, combined_z / len * step, false);
}

/// Slow left-right oscillation (not a full orbit) used by a ranged attacker while holding
/// position and firing - "slightly strafe left and right", not circling. Smooth (sine-based,
/// not a hard flip) so it reads as a subtle sidestep rather than a sudden direction reversal.
const RANGED_STRAFE_SPEED_BPS: f64 = 1.5;
const RANGED_STRAFE_PERIOD_TICKS: f64 = 50.0;

fn strafe_lightly(entity: &mut Entity, world: &World, width: f64, height: f64, target_pos: DVec3) {
    let dx = entity.position.x - target_pos.x;
    let dz = entity.position.z - target_pos.z;
    let radius = (dx * dx + dz * dz).sqrt().max(0.5);
    let phase = entity.ticks_existed as f64 / RANGED_STRAFE_PERIOD_TICKS * std::f64::consts::TAU;
    let direction = phase.sin();
    let (tangent_x, tangent_z) = (-dz / radius, dx / radius);

    let step = RANGED_STRAFE_SPEED_BPS / TICKS_PER_SECOND * direction;
    physics::move_horizontal(entity, world, width, height, tangent_x * step, tangent_z * step, false);
}

/// Same slow side-to-side wiggle as `strafe_lightly` above, blended with a radial correction
/// back toward `orbit_radius` - same reasoning as `strafe_around_target`'s own correction, so a
/// `LightStrafe` mob still backs off if it ends up standing right on the target instead of just
/// wiggling in place there.
fn strafe_lightly_toward_radius(entity: &mut Entity, world: &World, width: f64, height: f64, target_pos: DVec3, orbit_radius: f64) {
    let dx = entity.position.x - target_pos.x;
    let dz = entity.position.z - target_pos.z;
    let radius = (dx * dx + dz * dz).sqrt().max(0.5);
    let phase = entity.ticks_existed as f64 / RANGED_STRAFE_PERIOD_TICKS * std::f64::consts::TAU;
    let tangential_direction = phase.sin();
    let (tangent_x, tangent_z) = (-dz / radius * tangential_direction, dx / radius * tangential_direction);
    let radial_bias = ((orbit_radius - radius) / orbit_radius.max(0.5)).clamp(-1.0, 1.0);
    let (radial_x, radial_z) = (dx / radius * radial_bias, dz / radius * radial_bias);

    let combined_x = tangent_x + radial_x;
    let combined_z = tangent_z + radial_z;
    let len = (combined_x * combined_x + combined_z * combined_z).sqrt();
    if len < 1e-6 {
        return;
    }

    let step = RANGED_STRAFE_SPEED_BPS / TICKS_PER_SECOND;
    physics::move_horizontal(entity, world, width, height, combined_x / len * step, combined_z / len * step, false);
}

/// Deflection angles (degrees) tried in order when a direct step is fully blocked - small
/// angles first so a mob mostly still heads toward the target while skirting a thin
/// obstacle (like a single column of iron bars), wider angles as a last resort.
const DEFLECTION_ANGLES_DEG: [f64; 6] = [30.0, -30.0, 60.0, -60.0, 90.0, -90.0];

/// Moves `entity` one tick's worth of distance directly toward `target_pos` (XZ only - Y is
/// handled separately by `physics::apply_gravity`) and turns to face it. Horizontal movement
/// goes through `physics::move_horizontal`; if the direct line is fully blocked, a few
/// deflected angles are tried instead (see `DEFLECTION_ANGLES_DEG`) so a mob can slip past a
/// thin obstacle rather than just standing there pressed against it. This is a reactive
/// nudge, not real pathfinding (none exists in this codebase) - it won't solve a maze, but
/// it stops mobs getting permanently stuck on things like a single iron-bar block.
pub fn steer_toward(entity: &mut Entity, world: &World, width: f64, height: f64, target_pos: DVec3, speed_bps: f64, allow_jump: bool) {
    let dx = target_pos.x - entity.position.x;
    let dz = target_pos.z - entity.position.z;
    let horizontal_distance = (dx * dx + dz * dz).sqrt();

    // Faces wherever it's actually walking (`target_pos` here is a path waypoint most of the
    // time, not the real combat target - see `apply_movement_style`'s `steer_pos`), not the
    // player - a mob crossing the room to reach a staircase should visibly look toward the
    // staircase route while doing so, not dead-stare at the player the whole way there. `ai/mod.rs`
    // overrides this back to the real target afterward only once the mob is actually in attack
    // range - see the comment there.
    entity.yaw = yaw_towards(dx, dz);
    entity.pitch = pitch_towards(target_pos.y - entity.position.y, horizontal_distance);
    if horizontal_distance < 1e-4 {
        return;
    }

    let step = (speed_bps / TICKS_PER_SECOND).min(horizontal_distance);
    let move_x = dx / horizontal_distance * step;
    let move_z = dz / horizontal_distance * step;

    let (moved_x, moved_z) = physics::move_horizontal(entity, world, width, height, move_x, move_z, allow_jump);
    if moved_x || moved_z {
        return;
    }

    for angle_deg in DEFLECTION_ANGLES_DEG {
        let (deflected_x, deflected_z) = rotate_2d(move_x, move_z, angle_deg);
        let (moved_x, moved_z) = physics::move_horizontal(entity, world, width, height, deflected_x, deflected_z, allow_jump);
        if moved_x || moved_z {
            break;
        }
    }
}

/// Rotates a 2D (X/Z) vector by `degrees`.
fn rotate_2d(x: f64, z: f64, degrees: f64) -> (f64, f64) {
    let radians = degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    (x * cos - z * sin, x * sin + z * cos)
}

/// Turns to face `target_pos` (both yaw and pitch) without approaching/retreating. `target_pos`
/// is assumed to already be a feet position at roughly the same eye-height convention as
/// `entity` (both this codebase's mobs and players are approximated at the same eye height
/// elsewhere - see `perception.rs`), so a plain feet-to-feet Y difference is enough to pitch
/// toward eye level without needing to add/cancel a shared offset.
///
/// There's no look-only packet in this codebase's protocol layer (only `EntityTeleport`, sent
/// automatically by `Entity::tick` whenever position changes) - so a negligible,
/// direction-alternating vertical nudge is used purely to trigger that broadcast without
/// causing net drift. Safe to call even when something else already moved `entity` this tick
/// (the position-change broadcast already fires; this just adds an unnoticeable extra nudge).
pub fn face_toward(entity: &mut Entity, target_pos: DVec3) {
    let dx = target_pos.x - entity.position.x;
    let dz = target_pos.z - entity.position.z;
    let horizontal_distance = (dx * dx + dz * dz).sqrt();

    entity.yaw = yaw_towards(dx, dz);
    entity.pitch = pitch_towards(target_pos.y - entity.position.y, horizontal_distance);

    let jitter = if entity.ticks_existed % 2 == 0 { 0.0005 } else { -0.0005 };
    entity.position.y += jitter;
}

/// Converts an XZ direction into vanilla Minecraft yaw degrees (0 = south/+Z, 90 = west/-X,
/// 180 = north/-Z, 270 = east/+X), normalized to [0, 360). Also used by `projectile.rs` to
/// orient a fired arrow along its velocity.
pub fn yaw_towards(dx: f64, dz: f64) -> f32 {
    let yaw = (-dx).atan2(dz).to_degrees() as f32;
    if yaw < 0.0 { yaw + 360.0 } else { yaw }
}

/// Converts a vertical direction (`dy`) and horizontal distance into vanilla pitch degrees
/// (positive = looking down, negative = looking up). Used by `projectile.rs` for arrow
/// orientation and by `face_toward` above to tilt a mob's head toward its target.
pub fn pitch_towards(dy: f64, horizontal_distance: f64) -> f32 {
    (-dy.atan2(horizontal_distance)).to_degrees() as f32
}

/// Picks a random point within ~3 blocks of `spawn_origin` for idle wandering.
pub fn pick_idle_wander_target(spawn_origin: DVec3) -> DVec3 {
    let mut rng = rand::rng();
    let angle = rng.random::<f64>() * std::f64::consts::TAU;
    let radius = rng.random::<f64>() * 3.0;
    DVec3::new(
        spawn_origin.x + radius * angle.cos(),
        spawn_origin.y,
        spawn_origin.z + radius * angle.sin(),
    )
}

/// Randomized 60-120 tick interval between idle-wander destination picks.
pub fn random_idle_wander_interval() -> u32 {
    rand::rng().random_range(60..=120)
}

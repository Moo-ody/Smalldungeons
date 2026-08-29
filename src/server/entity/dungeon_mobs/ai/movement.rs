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
    // Phase 3+: Circling { .. }, Strafe { .. }, SprintJump { .. } - enum extends here,
    // the pipeline in ai/mod.rs does not need to change when they're added.
}

/// Ticks per second, matching the server's fixed 20 TPS main loop.
const TICKS_PER_SECOND: f64 = 20.0;

pub fn apply_movement_style(
    entity: &mut Entity,
    world: &World,
    width: f64,
    height: f64,
    style: MovementStyle,
    target_pos: DVec3,
    speed_bps: f64,
) {
    match style {
        MovementStyle::Approach => steer_toward(entity, world, width, height, target_pos, speed_bps),
        MovementStyle::MaintainDistance { preferred, tolerance } => {
            let distance = entity.position.distance_to(&target_pos);
            if distance > preferred + tolerance {
                steer_toward(entity, world, width, height, target_pos, speed_bps);
            } else {
                face_toward(entity, target_pos);
            }
        }
        MovementStyle::HybridMeleeRanged { melee_range, ranged_preferred, ranged_tolerance } => {
            let distance = entity.position.distance_to(&target_pos);
            if distance < melee_range || distance > ranged_preferred + ranged_tolerance {
                steer_toward(entity, world, width, height, target_pos, speed_bps);
            } else {
                face_toward(entity, target_pos);
            }
        }
    }
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
pub fn steer_toward(entity: &mut Entity, world: &World, width: f64, height: f64, target_pos: DVec3, speed_bps: f64) {
    let dx = target_pos.x - entity.position.x;
    let dz = target_pos.z - entity.position.z;
    let horizontal_distance = (dx * dx + dz * dz).sqrt();

    entity.yaw = yaw_towards(dx, dz);
    if horizontal_distance < 1e-4 {
        return;
    }

    let step = (speed_bps / TICKS_PER_SECOND).min(horizontal_distance);
    let move_x = dx / horizontal_distance * step;
    let move_z = dz / horizontal_distance * step;

    let (moved_x, moved_z) = physics::move_horizontal(entity, world, width, height, move_x, move_z);
    if moved_x || moved_z {
        return;
    }

    for angle_deg in DEFLECTION_ANGLES_DEG {
        let (deflected_x, deflected_z) = rotate_2d(move_x, move_z, angle_deg);
        let (moved_x, moved_z) = physics::move_horizontal(entity, world, width, height, deflected_x, deflected_z);
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

/// Turns to face `target_pos` without approaching/retreating. There's no look-only packet
/// in this codebase's protocol layer (only `EntityTeleport`, sent automatically by
/// `Entity::tick` whenever position changes) - so a negligible, direction-alternating
/// vertical nudge is used purely to trigger that broadcast without causing net drift.
pub fn face_toward(entity: &mut Entity, target_pos: DVec3) {
    let dx = target_pos.x - entity.position.x;
    let dz = target_pos.z - entity.position.z;
    entity.yaw = yaw_towards(dx, dz);

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
/// (positive = looking down, negative = looking up). Only used by `projectile.rs` today -
/// mobs never need to look up/down since all steering here is horizontal-only.
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

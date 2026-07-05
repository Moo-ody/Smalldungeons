//! Shared basic physics for dungeon mobs: gravity/ground collision, wall/player collision
//! during horizontal movement, and soft mob-mob separation. Applied unconditionally to every
//! spawned dungeon mob (even archetypes whose behavior isn't implemented yet) - falling and
//! not clipping through walls/players/each other is a baseline physical property, not an AI
//! behavior choice.

use crate::server::block::block_collision::{check_block_collisions, is_liquid};
use crate::server::entity::dungeon_mobs::mob_type::MobBaseKind;
use crate::server::entity::entity::{Entity, EntityId};
use crate::server::utils::aabb::AABB;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;

/// Matches vanilla `EntityLivingBase` gravity (blocks/tick^2).
const GRAVITY_PER_TICK: f64 = 0.08;
/// Simple fall-speed cap so a mob dropped from a great height doesn't tunnel through thin floors.
const TERMINAL_VELOCITY: f64 = -3.92;

/// Much weaker gravity while submerged, plus a small constant upward drift - net effect is a
/// slow float/rise rather than sinking like a stone or free-falling through the liquid, so a
/// mob that falls into water/lava can drift back up and swim out instead of being stuck.
const LIQUID_GRAVITY_PER_TICK: f64 = 0.02;
const LIQUID_BUOYANCY_PER_TICK: f64 = 0.03;
const LIQUID_VERTICAL_SPEED_CAP: f64 = 0.2;

/// How far a mob can auto-step up onto a short obstruction (slab, stair, carpet, etc.)
/// instead of treating it as a full wall - matches vanilla's ~0.5-0.6 block step height.
const STEP_HEIGHT: f64 = 0.6;

/// Approximate per-model collision box (width, height) - this codebase doesn't track exact
/// per-mob hitboxes anywhere live (only the dead `old_entity` code did), so these are
/// reasonable vanilla-ish approximations, close enough for basic collision.
pub fn size_for(base_kind: MobBaseKind) -> (f64, f64) {
    match base_kind {
        MobBaseKind::Enderman => (0.6, 2.9),
        _ => (0.6, 1.95),
    }
}

fn aabb_at(position: DVec3, width: f64, height: f64) -> AABB {
    AABB::from_height_width(height, width).offset(position)
}

/// Whether solid ground is directly beneath `position` (a thin slab just under the feet).
pub fn is_on_ground(world: &World, position: DVec3, width: f64) -> bool {
    let probe = aabb_at(DVec3::new(position.x, position.y - 0.1, position.z), width, 0.1);
    check_block_collisions(world, &probe)
}

/// Whether `position` (feet level) is inside a liquid block - used to switch to buoyancy
/// instead of normal gravity.
pub fn is_in_liquid(world: &World, position: DVec3) -> bool {
    let block = world.get_block_at(position.x.floor() as i32, position.y.floor() as i32, position.z.floor() as i32);
    is_liquid(block)
}

/// Whether a mob's box at `position` overlaps any currently connected player's hitbox.
fn overlaps_any_player(world: &World, position: DVec3, width: f64, height: f64) -> bool {
    let aabb = aabb_at(position, width, height);
    world.players.values().any(|player| aabb.intersects(&player.collision_aabb()))
}

/// Applies gravity (or buoyancy, if submerged) for one tick: falls when not grounded, stops
/// falling (without clipping through the floor) when it is, and drifts back toward the
/// surface instead of sinking when in water/lava. Updates `entity.on_ground` to match.
pub fn apply_gravity(entity: &mut Entity, world: &World, width: f64, height: f64) {
    let submerged = is_in_liquid(world, entity.position);
    let grounded = !submerged && is_on_ground(world, entity.position, width);
    entity.on_ground = grounded;

    if submerged {
        entity.velocity.y = (entity.velocity.y - LIQUID_GRAVITY_PER_TICK + LIQUID_BUOYANCY_PER_TICK)
            .clamp(-LIQUID_VERTICAL_SPEED_CAP, LIQUID_VERTICAL_SPEED_CAP);
    } else if grounded && entity.velocity.y < 0.0 {
        entity.velocity.y = 0.0;
    } else if !grounded {
        entity.velocity.y = (entity.velocity.y - GRAVITY_PER_TICK).max(TERMINAL_VELOCITY);
    }

    if entity.velocity.y == 0.0 {
        return;
    }

    let candidate_y = entity.position.y + entity.velocity.y;
    let candidate = DVec3::new(entity.position.x, candidate_y, entity.position.z);
    if check_block_collisions(world, &aabb_at(candidate, width, height)) {
        entity.velocity.y = 0.0;
    } else {
        entity.position.y = candidate_y;
    }
}

/// Moves `entity` by `(dx, dz)` horizontally, testing each axis independently against solid
/// blocks and player hitboxes (simple axis-separated "slide" collision - won't perfectly hug
/// diagonal walls, but stops mobs from ever walking through terrain or into a player). If an
/// axis is blocked at the current height, tries auto-stepping up onto it first (see
/// `STEP_HEIGHT`) before giving up on that axis - this is what makes slabs/stairs/carpets
/// climbable instead of a full wall. Returns `(moved_x, moved_z)` so callers (see
/// `movement::steer_toward`'s obstacle deflection) can tell whether the step was blocked.
pub fn move_horizontal(entity: &mut Entity, world: &World, width: f64, height: f64, dx: f64, dz: f64) -> (bool, bool) {
    let moved_x = if dx != 0.0 { try_move_axis(entity, world, width, height, dx, 0.0) } else { false };
    let moved_z = if dz != 0.0 { try_move_axis(entity, world, width, height, 0.0, dz) } else { false };
    (moved_x, moved_z)
}

fn try_move_axis(entity: &mut Entity, world: &World, width: f64, height: f64, dx: f64, dz: f64) -> bool {
    let flat_candidate = DVec3::new(entity.position.x + dx, entity.position.y, entity.position.z + dz);
    if !is_blocked(world, flat_candidate, width, height) {
        entity.position.x = flat_candidate.x;
        entity.position.z = flat_candidate.z;
        return true;
    }

    let stepped_candidate = DVec3::new(flat_candidate.x, entity.position.y + STEP_HEIGHT, flat_candidate.z);
    if !is_blocked(world, stepped_candidate, width, height) {
        entity.position.x = stepped_candidate.x;
        entity.position.y = stepped_candidate.y;
        entity.position.z = stepped_candidate.z;
        return true;
    }

    false
}

fn is_blocked(world: &World, position: DVec3, width: f64, height: f64) -> bool {
    check_block_collisions(world, &aabb_at(position, width, height)) || overlaps_any_player(world, position, width, height)
}

/// Soft separation impulse away from any other dungeon mob whose hitbox overlaps this one's -
/// vanilla mobs continuously push each other apart rather than hard-blocking, so this returns
/// a small horizontal nudge to apply via `move_horizontal` rather than a hard collision test.
pub fn separation_push(world: &World, entity_id: EntityId, position: DVec3, width: f64) -> (f64, f64) {
    let mut push_x = 0.0;
    let mut push_z = 0.0;

    for (other_id, (other_entity, _)) in &world.entities {
        if *other_id == entity_id || !world.entity_mob_ai.contains_key(other_id) {
            continue;
        }

        let dx = position.x - other_entity.position.x;
        let dz = position.z - other_entity.position.z;
        let distance = (dx * dx + dz * dz).sqrt();
        let min_distance = width;
        if distance < min_distance && distance > 1e-4 {
            let overlap = (min_distance - distance) * 0.5;
            push_x += dx / distance * overlap;
            push_z += dz / distance * overlap;
        }
    }

    (push_x, push_z)
}

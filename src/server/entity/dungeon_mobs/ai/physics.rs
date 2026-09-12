//! Shared basic physics for dungeon mobs: gravity/ground collision, wall/player collision
//! during horizontal movement, and soft mob-mob separation. Applied unconditionally to every
//! spawned dungeon mob (even archetypes whose behavior isn't implemented yet) - falling and
//! not clipping through walls/players/each other is a baseline physical property, not an AI
//! behavior choice.

use crate::server::block::block_collision::{check_block_collisions, get_block_aabb, is_liquid, stair_top_half};
use crate::server::block::blocks::Blocks;
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

/// Whether `position` (feet level) is inside lava specifically, not just any liquid - used to
/// instantly kill a mob that falls into or touches it (e.g. Lava Ravine/Lava Pit's terrain
/// hazard), unlike water, which `is_in_liquid`'s buoyancy handling already lets a mob float/swim
/// back out of.
pub fn is_in_lava(world: &World, position: DVec3) -> bool {
    let block = world.get_block_at(position.x.floor() as i32, position.y.floor() as i32, position.z.floor() as i32);
    matches!(block, Blocks::Lava { .. } | Blocks::FlowingLava { .. })
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
/// `allow_jump` gates the full-height-ledge jump fallback below (see its own comment) - true
/// only for movement that's actually executing a resolved pathfinding waypoint, per request:
/// mobs should jump because the path they're following calls for it, not as a general reflex
/// any time direct-line steering (idle wander, leash return, the first-sight nudge, a
/// no-path-found fallback) happens to bump into a solid ledge.
pub fn move_horizontal(entity: &mut Entity, world: &World, width: f64, height: f64, dx: f64, dz: f64, allow_jump: bool) -> (bool, bool) {
    let moved_x = if dx != 0.0 { try_move_axis(entity, world, width, height, dx, 0.0, allow_jump) } else { false };
    let moved_z = if dz != 0.0 { try_move_axis(entity, world, width, height, 0.0, dz, allow_jump) } else { false };
    (moved_x, moved_z)
}

fn try_move_axis(entity: &mut Entity, world: &World, width: f64, height: f64, dx: f64, dz: f64, allow_jump: bool) -> bool {
    let flat_candidate = DVec3::new(entity.position.x + dx, entity.position.y, entity.position.z + dz);

    // Treated as a full obstruction (refuses the whole axis, same as a genuine wall) rather than
    // just rejecting the flat candidate below and falling through to the step-up attempt - the
    // step-up would also succeed here (there's nothing solid up there to actually block it,
    // just open air over the recess), so rejecting only the flat candidate wouldn't have stopped
    // the sink, only delayed it by one stutter-step. Only checked while grounded, same reasoning
    // `safe_to_step` already uses (a mob already falling for some unrelated reason shouldn't have
    // its horizontal drift frozen by this).
    if entity.on_ground && is_decorative_stair_recess(world, width, entity.position, flat_candidate, dx, dz) {
        return false;
    }

    if !is_blocked(world, flat_candidate, width, height) && safe_to_step(world, entity, flat_candidate, width) {
        entity.position.x = flat_candidate.x;
        entity.position.z = flat_candidate.z;
        return true;
    }

    // A real ascending staircase (confirmed against actual dungeon block data: consecutive
    // `Stairs` blocks, each one column over AND one full block higher than the last, e.g.
    // `Raccoon` room x=5..8 z=6) rises a full 1.0 block per tread, not `STEP_HEIGHT` (0.6) -
    // vanilla players/mobs climb this smoothly because the game's real collision resolves
    // against each stair's true diagonal shape continuously as they walk across it, but this
    // codebase simplifies every stair/slab to one flat half-height box per column
    // (`block_collision::get_stair_aabb`/`half_height_aabb`), so a mob already standing on one
    // tread meets the next tread's box as a sudden full-block-ish wall, not a gradual ramp.
    // `STEP_HEIGHT` alone can clear the *first* rise (flat ground onto a tread), never a
    // *second* consecutive one - hence "climbs one stair, not a flight". Compensate with a
    // taller step attempt, but only when what's actually ahead is itself a partial-height
    // block (stair/slab/carpet) rather than a genuine full 1-block wall/ledge - a full wall
    // still isn't auto-climbable this way, matching vanilla (that needs an actual jump).
    let step_height = if obstruction_is_partial_height(world, flat_candidate) {
        STAIR_STEP_HEIGHT
    } else {
        STEP_HEIGHT
    };
    let stepped_candidate = DVec3::new(flat_candidate.x, entity.position.y + step_height, flat_candidate.z);
    if !is_blocked(world, stepped_candidate, width, height) && safe_to_step(world, entity, stepped_candidate, width) {
        entity.position.x = stepped_candidate.x;
        entity.position.y = stepped_candidate.y;
        entity.position.z = stepped_candidate.z;
        return true;
    }

    // Neither an instant step nor a stair-step cleared it - if `allow_jump` (this is a resolved
    // pathfinding waypoint that actually calls for it, not a direct-line steering reflex - see
    // `move_horizontal`'s doc comment), this is a genuine full-height obstruction (not a
    // stair/slab/carpet, already handled above), and the mob is currently grounded, jump instead
    // of just giving up: matches vanilla mobs' ability to auto-jump a 1-block ledge/wall while
    // navigating, which nothing here did before (the two step attempts above only ever clear up
    // to ~1.05 blocks *without* leaving the ground - a real solid full-block ledge stays taller
    // than that no matter which of them is tried). Unlike the step attempts, this is a genuine
    // velocity impulse, not an instant teleport - `apply_gravity` (already run once per tick
    // before movement, see `ai/mod.rs`) carries the resulting arc up and back down over several
    // ticks just like a real jump, which also naturally prevents chaining multiple jumps in the
    // same tick or before landing again (`entity.on_ground` only goes back to `true` once it
    // does) without needing any separate cooldown state.
    if allow_jump && entity.on_ground && !obstruction_is_partial_height(world, flat_candidate) {
        entity.velocity.y = JUMP_VELOCITY;
    }

    false
}

/// Whether stepping flat onto `candidate` (same Y as the mob's current position) would walk it
/// down into a *decorative* stair recessed half a block below the surrounding floor, rather
/// than a genuine lower path (a real descending staircase, or an actual drop/ledge) - confirmed
/// by explicit report with a screenshot: mobs were partially sinking into decorative stairs
/// recessed into an otherwise-flat floor. The existing flat-Y step check only tests for a
/// *collision* at the current height, and a lower-half stair's real collision box
/// (`[y, y+0.5]` - see `block_collision::get_stair_aabb`) sits entirely below a mob currently
/// standing at `y+1` on a normal full-height floor block, so the flat-Y box floats clear over
/// it with no collision at all - the move succeeded, and gravity alone quietly sank the mob
/// into the recess over the next few ticks (`is_on_ground`'s thin probe never reached down to
/// the stair's real, lower surface either).
///
/// Distinguishes the two cases the same way a person looking at it would: is the floor *right
/// after this dip*, one more step past it in the same direction of travel, back at the height
/// the mob's currently standing at? If so, this one recessed stair is an isolated decorative dip
/// in an otherwise-flat floor - refuse the flat step (steering's own deflection-angle fallback
/// routes around it instead). If the floor beyond keeps trending down (a real staircase) or
/// opens onto nothing (a genuine ledge), this returns `false` and movement proceeds exactly as
/// it already did - upward stair-climbing (`STAIR_STEP_HEIGHT` above) is untouched either way,
/// since that's a different code path entirely.
fn is_decorative_stair_recess(world: &World, width: f64, current_pos: DVec3, candidate: DVec3, dx: f64, dz: f64) -> bool {
    // The layer the mob is presumably standing *on top of* right now (one block below its own
    // feet) - this only means "recessed below the current floor" relative to that specific
    // layer, so this is the one to check the target column against, not the candidate's own Y.
    let current_floor_y = current_pos.y.floor() as i32 - 1;
    let target_x = candidate.x.floor() as i32;
    let target_z = candidate.z.floor() as i32;

    let target_block = world.get_block_at(target_x, current_floor_y, target_z);
    let Some(top_half) = stair_top_half(target_block) else { return false };
    if top_half {
        // An upper-half stair's collision top is at the SAME height a normal full block at this
        // layer would have (`y+1`) - not a recess at all, nothing to catch here.
        return false;
    }

    // `f64::signum()` returns `1.0` (not `0.0`) for an input of exactly `0.0` - `move_horizontal`
    // always calls this one axis at a time (the other delta is exactly `0.0`), so a naive
    // `.signum()` on both would incorrectly offset the probe diagonally on the axis that isn't
    // actually moving.
    let step_x = if dx != 0.0 { dx.signum() } else { 0.0 };
    let step_z = if dz != 0.0 { dz.signum() } else { 0.0 };

    let further = DVec3::new(candidate.x + step_x, current_pos.y, candidate.z + step_z);
    let probe = aabb_at(DVec3::new(further.x, further.y - 0.1, further.z), width, 0.1);
    check_block_collisions(world, &probe)
}

/// Vanilla's real initial jump velocity (`EntityLivingBase.jump()`'s `motionY = 0.42F`, no
/// jump-boost effect applied here).
const JUMP_VELOCITY: f64 = 0.42;

/// Forces a jump (same impulse `try_move_axis`'s own ledge-jump fallback uses) if `entity` is
/// currently grounded - a no-op mid-air (matches vanilla: you can't jump again until you land).
/// Used for a purely cosmetic "bunny-hop while charging in" effect (see `ai/mod.rs`'s
/// `hops_while_approaching`), unlike the ledge-jump above, which only fires reactively when
/// something's actually blocking the way.
pub fn try_jump(entity: &mut Entity) {
    if entity.on_ground {
        entity.velocity.y = JUMP_VELOCITY;
    }
}

/// Taller step-up height for climbing onto another partial-height (stair/slab/carpet) block
/// specifically - see the comment in `try_move_axis` for why more than `STEP_HEIGHT` is needed
/// here. 1.05 clears a full 1.0-block tread rise with a small margin.
const STAIR_STEP_HEIGHT: f64 = 1.05;

/// Whether the block directly at `candidate`'s feet position has a partial-height collision
/// box (a stair, slab, or carpet - anything shorter than a full 1x1x1 cube), as opposed to a
/// genuine full-block wall/ledge or open air. Only the block at the mob's own feet height is
/// checked, matching what `is_blocked`'s flat-candidate test itself just failed against.
fn obstruction_is_partial_height(world: &World, candidate: DVec3) -> bool {
    let x = candidate.x.floor() as i32;
    let y = candidate.y.floor() as i32;
    let z = candidate.z.floor() as i32;
    let block = world.get_block_at(x, y, z);
    match get_block_aabb(block, x, y, z) {
        Some(aabb) => (aabb.max.y - aabb.min.y) < 1.0,
        None => false,
    }
}

/// Real vanilla mobs generally avoid voluntarily walking off a drop that would deal fall
/// damage (past 3 blocks) - the same threshold `ai::pathfinding`'s `MAX_SAFE_DROP` already uses
/// to keep A* from routing *through* a big gap. This is the belt-and-suspenders half of that:
/// it applies at the base movement layer itself, so the same protection also covers whatever
/// doesn't go through a computed path at all - the direct-steer fallback when no path exists
/// (different room, search budget exhausted, genuinely no route), plus idle wander and leash
/// return-to-spawn, which don't call into pathfinding in the first place.
const MAX_SAFE_FALL: f64 = 3.0;

/// Whether solid ground exists anywhere within `MAX_SAFE_FALL` blocks below `candidate` - see
/// `safe_to_step`.
fn has_safe_landing(world: &World, candidate: DVec3, width: f64) -> bool {
    let probe = AABB::from_height_width(MAX_SAFE_FALL, width)
        .offset(DVec3::new(candidate.x, candidate.y - MAX_SAFE_FALL, candidate.z));
    check_block_collisions(world, &probe)
}

/// Gates `try_move_axis`'s two step attempts on fall safety, but only while the mob is
/// currently grounded - a mob already falling for some other reason (knockback, spawned
/// mid-air) shouldn't have its horizontal drift frozen mid-fall just because there's nothing
/// below it; that's a pre-existing, unrelated situation this isn't meant to touch.
fn safe_to_step(world: &World, entity: &Entity, candidate: DVec3, width: f64) -> bool {
    !entity.on_ground || has_safe_landing(world, candidate, width)
}

fn is_blocked(world: &World, position: DVec3, width: f64, height: f64) -> bool {
    check_block_collisions(world, &aabb_at(position, width, height)) || overlaps_any_player(world, position, width, height)
}

/// Soft separation impulse away from any other dungeon mob whose hitbox overlaps this one's -
/// vanilla mobs continuously push each other apart rather than hard-blocking, so this returns
/// a small horizontal nudge to apply via `move_horizontal` rather than a hard collision test.
///
/// Looks up candidates through `world.mob_spatial_grid` (rebuilt once per tick, see its own doc
/// comment) instead of scanning every entity in the world - the push only ever matters within
/// `width` (well under one block), so the 3x3 neighborhood of `MOB_GRID_CELL_SIZE`-sized cells
/// around this mob's own position is guaranteed to contain every mob that could possibly push
/// it. At dungeon-mob-count scale (hundreds to low thousands) the old global scan was an
/// O(mobs^2) per-tick cost - by far the dominant one in a packed stress-test spawn - this makes
/// it O(local density) per mob instead.
pub fn separation_push(world: &World, entity_id: EntityId, position: DVec3, width: f64) -> (f64, f64) {
    let mut push_x = 0.0;
    let mut push_z = 0.0;

    let (cell_x, cell_z) = crate::server::world::mob_grid_cell(position);
    for dx_cell in -1..=1 {
        for dz_cell in -1..=1 {
            let Some(candidates) = world.mob_spatial_grid.get(&(cell_x + dx_cell, cell_z + dz_cell)) else { continue };
            for &other_id in candidates {
                if other_id == entity_id {
                    continue;
                }
                let Some((other_entity, _)) = world.entities.get(&other_id) else { continue };

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
        }
    }

    (push_x, push_z)
}

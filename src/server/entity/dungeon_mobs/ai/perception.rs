//! Vision/perception: "can this mob see a player" and the line-of-sight check that backs it.
//!
//! No such helper existed anywhere in the codebase before this - only bespoke per-projectile
//! collision routines with different goals (see module docs on `has_line_of_sight`).

use crate::server::block::block_collision::is_block_passable;
use crate::server::player::player::ClientId;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;

/// Approximate eye height used for both the mob and the player when checking vision/LOS.
/// Good enough for gameplay purposes; not meant to match each mob model's exact height.
const EYE_HEIGHT: f64 = 1.6;

/// Finds the nearest player within `vision_range` blocks of `mob_pos`, optionally requiring
/// an unobstructed line of sight. Returns `None` if nobody qualifies.
pub fn find_vision_target(
    world: &World,
    mob_pos: DVec3,
    vision_range: f64,
    requires_los: bool,
) -> Option<ClientId> {
    let mob_eye = DVec3::new(mob_pos.x, mob_pos.y + EYE_HEIGHT, mob_pos.z);

    world.players.iter()
        .filter_map(|(client_id, player)| {
            let player_eye = DVec3::new(player.position.x, player.position.y + EYE_HEIGHT, player.position.z);
            let distance = mob_eye.distance_to(&player_eye);
            if distance > vision_range {
                return None;
            }
            if requires_los && !has_line_of_sight(world, mob_eye, player_eye) {
                return None;
            }
            Some((*client_id, distance))
        })
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(client_id, _)| client_id)
}

/// `has_line_of_sight` for two feet-position points (mob, then player) - applies the same
/// `EYE_HEIGHT` offset `find_vision_target` uses for initial acquisition, so a mob's *ongoing*
/// combat LOS (re-checked every tick - see `ai/mod.rs`) uses the exact same definition of
/// "can see" as the check that let it notice the player in the first place.
pub fn has_los_to_player(world: &World, mob_pos: DVec3, player_pos: DVec3) -> bool {
    let mob_eye = DVec3::new(mob_pos.x, mob_pos.y + EYE_HEIGHT, mob_pos.z);
    let player_eye = DVec3::new(player_pos.x, player_pos.y + EYE_HEIGHT, player_pos.z);
    has_line_of_sight(world, mob_eye, player_eye)
}

/// Simple fixed-step (~0.5 block) sampler along the segment `from -> to`, checking each
/// sampled point's block for passability. Deliberately not the DDA voxel-traversal used by
/// `ender_pearl.rs` - that solves "find the precise point a physical projectile first hits
/// something," a different problem from this yes/no visibility check.
pub fn has_line_of_sight(world: &World, from: DVec3, to: DVec3) -> bool {
    let delta = to - from;
    let distance = (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt();
    if distance < 1e-4 {
        return true;
    }

    let steps = (distance / 0.5).ceil().max(1.0) as i32;
    let step = DVec3::new(delta.x / steps as f64, delta.y / steps as f64, delta.z / steps as f64);

    let mut pos = from;
    for _ in 0..steps {
        pos = pos + step;
        let block = world.get_block_at(pos.x.floor() as i32, pos.y.floor() as i32, pos.z.floor() as i32);
        if !is_block_passable(block) {
            return false;
        }
    }
    true
}

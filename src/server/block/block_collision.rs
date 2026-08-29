use crate::server::block::blocks::Blocks;
use crate::server::block::block_parameter::StairDirection;
use crate::server::utils::aabb::AABB;
use crate::server::utils::dvec3::DVec3;

/// Get the collision AABB for a block at the given position
/// Returns None if the block is passable (air, liquids, etc.)
pub fn get_block_aabb(block: Blocks, x: i32, y: i32, z: i32) -> Option<AABB> {
    let base_min = DVec3::new(x as f64, y as f64, z as f64);
    let base_max = DVec3::new((x + 1) as f64, (y + 1) as f64, (z + 1) as f64);

    match block {
        Blocks::Air => None,

        // Liquids never block movement - mobs (and anything else) can swim into/through
        // them instead of treating them as a solid wall. Buoyancy/swimming physics is
        // handled separately in `dungeon_mobs::ai::physics` via `is_liquid`.
        Blocks::FlowingWater { .. }
        | Blocks::StillWater { .. }
        | Blocks::FlowingLava { .. }
        | Blocks::Lava { .. } => None,

        // Torches are thin decoration attached to a surface, no real hitbox in vanilla -
        // without this they fell through to the full-cube default below, so pearls (and
        // anything else using this for collision) would stop dead next to a torch.
        Blocks::Torch { .. }
        | Blocks::UnlitRedstoneTorch { .. }
        | Blocks::RedstoneTorch { .. } => None,

        // Single slabs are half-height "climbable" obstructions, not a full 1x1 block -
        // combined with `physics::move_horizontal`'s step-up, this lets mobs walk onto them
        // instead of being stopped cold. Double slabs stay full-height (handled by the `_`
        // catch-all below).
        Blocks::StoneSlab { top_half, .. }
        | Blocks::WoodenSlab { top_half, .. }
        | Blocks::NewStoneSlab { top_half, .. } => Some(half_height_aabb(top_half, base_min, base_max)),

        // Stairs: same reasoning as slabs. `get_stair_aabb` already simplifies the shape to a
        // half-block (direction doesn't change the returned box) - it just wasn't wired into
        // this match before, so stairs fell through to the full-cube default.
        Blocks::OakStairs { direction, top_half }
        | Blocks::StoneStairs { direction, top_half }
        | Blocks::BrickStairs { direction, top_half }
        | Blocks::StoneBrickStairs { direction, top_half }
        | Blocks::NetherbrickStairs { direction, top_half }
        | Blocks::SandstoneStairs { direction, top_half }
        | Blocks::SpruceStairs { direction, top_half }
        | Blocks::BirchStairs { direction, top_half }
        | Blocks::JungleStairs { direction, top_half }
        | Blocks::QuartzStairs { direction, top_half }
        | Blocks::AcaciaStairs { direction, top_half }
        | Blocks::DarkOakStairs { direction, top_half }
        | Blocks::RedSandstoneStairs { direction, top_half } => get_stair_aabb(direction, top_half, base_min, base_max),

        // Simplified collision: every other non-air block is a full 1x1x1 solid cube.
        _ => Some(AABB::new(base_min, base_max)),
    }
}

fn half_height_aabb(top_half: bool, base_min: DVec3, base_max: DVec3) -> AABB {
    let (min_y, max_y) = if top_half {
        (base_min.y + 0.5, base_max.y)
    } else {
        (base_min.y, base_min.y + 0.5)
    };
    AABB::new(
        DVec3::new(base_min.x, min_y, base_min.z),
        DVec3::new(base_max.x, max_y, base_max.z),
    )
}

/// Get AABB for stairs based on direction and top_half
fn get_stair_aabb(
    direction: StairDirection,
    top_half: bool,
    base_min: DVec3,
    base_max: DVec3,
) -> Option<AABB> {
    let half_y = if top_half {
        (base_min.y + 0.5, base_max.y)
    } else {
        (base_min.y, base_min.y + 0.5)
    };

    // Stairs have a complex shape - simplified to half-block for collision
    // The actual shape varies by direction, but for pearl collision we can approximate
    // as a half-block on the appropriate side
    let _ = direction; // direction intentionally unused - see comment above
    Some(AABB::new(
        DVec3::new(base_min.x, half_y.0, base_min.z),
        DVec3::new(base_max.x, half_y.1, base_max.z),
    ))
}

/// Whether a block is a liquid (water or lava, flowing or still) - used to trigger
/// buoyancy/swimming physics, distinct from `is_block_passable` (which is also true for
/// liquids, since they don't block movement, but true for plain air too).
pub fn is_liquid(block: Blocks) -> bool {
    matches!(
        block,
        Blocks::FlowingWater { .. } | Blocks::StillWater { .. } | Blocks::FlowingLava { .. } | Blocks::Lava { .. }
    )
}

/// Check if a block is passable for projectiles
pub fn is_block_passable(block: Blocks) -> bool {
    get_block_aabb(block, 0, 0, 0).is_none()
}

/// Checks if an AABB collides with any solid block in the world.
/// Iterates all blocks overlapped by the AABB and tests per-block collision.
pub fn check_block_collisions(
    world: &crate::server::world::World,
    aabb: &AABB,
) -> bool {
    let min_x = aabb.min.x.floor() as i32;
    let min_y = aabb.min.y.floor() as i32;
    let min_z = aabb.min.z.floor() as i32;
    let max_x = aabb.max.x.ceil() as i32;
    let max_y = aabb.max.y.ceil() as i32;
    let max_z = aabb.max.z.ceil() as i32;

    for x in min_x..max_x {
        for y in min_y..max_y {
            for z in min_z..max_z {
                let block = world.get_block_at(x, y, z);
                if let Some(block_aabb) = get_block_aabb(block, x, y, z) {
                    if aabb.intersects(&block_aabb) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Simple block-collision helper returning up to one AABB for a given block.
/// Length is 0 for non-colliding blocks (air, fluids, etc.).
pub fn block_collision(block: Blocks) -> ([AABB; 1], usize) {
    if let Some(aabb) = get_block_aabb(block, 0, 0, 0) {
        ([aabb], 1)
    } else {
        // Value is unused when len == 0, but keep a valid AABB for safety.
        (
            [AABB::new(
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 1.0, 1.0),
            )],
            0,
        )
    }
}

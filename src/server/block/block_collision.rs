use crate::server::block::blocks::Blocks;
use crate::server::block::block_parameter::StairDirection;
use crate::server::utils::aabb::AABB;
use crate::server::utils::dvec3::DVec3;

/// Get the collision AABB for a block at the given position
/// Returns None if the block is passable (air, water, etc.)
pub fn get_block_aabb(block: Blocks, x: i32, y: i32, z: i32) -> Option<AABB> {
    let base_min = DVec3::new(x as f64, y as f64, z as f64);
    let base_max = DVec3::new((x + 1) as f64, (y + 1) as f64, (z + 1) as f64);

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
        | Blocks::ActivatorRail { .. }
        | Blocks::DaylightSensor { .. }
        | Blocks::InvertedDaylightSensor { .. }
        | Blocks::Ladder { .. } => None,

        // Cobblestone walls - 0.125 to 0.875 (75% width, full height)
        Blocks::CobblestoneWalls { .. } => Some(AABB::new(
            DVec3::new(base_min.x + 0.125, base_min.y, base_min.z + 0.125),
            DVec3::new(base_max.x - 0.125, base_max.y, base_max.z - 0.125),
        )),

        // Fences - similar to walls
        Blocks::Fence
        | Blocks::NetherbrickFence
        | Blocks::SpruceFence
        | Blocks::BirchFence
        | Blocks::JungleFence
        | Blocks::DarkOakFence
        | Blocks::AcaicaFence => Some(AABB::new(
            DVec3::new(base_min.x + 0.125, base_min.y, base_min.z + 0.125),
            DVec3::new(base_max.x - 0.125, base_max.y, base_max.z - 0.125),
        )),

        // Iron bars - thin like glass panes
        Blocks::IronBars => Some(AABB::new(
            DVec3::new(base_min.x + 0.4375, base_min.y, base_min.z + 0.4375),
            DVec3::new(base_max.x - 0.4375, base_max.y, base_max.z - 0.4375),
        )),

        // Glass panes - extremely thin
        Blocks::GlassPane
        | Blocks::StainedGlassPane { .. } => Some(AABB::new(
            DVec3::new(base_min.x + 0.4375, base_min.y, base_min.z + 0.4375),
            DVec3::new(base_max.x - 0.4375, base_max.y, base_max.z - 0.4375),
        )),

        // Stone slabs - half height
        Blocks::StoneSlab { top_half, .. }
        | Blocks::NewStoneSlab { top_half, .. }
        | Blocks::WoodenSlab { top_half, .. } => {
            if top_half {
                Some(AABB::new(
                    DVec3::new(base_min.x, base_min.y + 0.5, base_min.z),
                    base_max,
                ))
            } else {
                Some(AABB::new(
                    base_min,
                    DVec3::new(base_max.x, base_min.y + 0.5, base_max.z),
                ))
            }
        }

        // Double slabs - full block
        Blocks::DoubleStoneSlab { .. }
        | Blocks::NewDoubleStoneSlab { .. }
        | Blocks::DoubleWoodenSlab { .. } => Some(AABB::new(base_min, base_max)),

        // Stairs - complex shape based on direction
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
        | Blocks::RedSandstoneStairs { direction, top_half } => {
            get_stair_aabb(direction, top_half, base_min, base_max)
        }

        // Trapdoors - vary by open/closed state
        Blocks::Trapdoor { open, top_half, .. }
        | Blocks::IronTrapdoor { open, top_half, .. } => {
            if open {
                None // Open trapdoor is passable
            } else if top_half {
                Some(AABB::new(
                    DVec3::new(base_min.x, base_min.y + 0.8125, base_min.z),
                    base_max,
                ))
            } else {
                Some(AABB::new(
                    base_min,
                    DVec3::new(base_max.x, base_min.y + 0.1875, base_max.z),
                ))
            }
        }

        // Doors - vary by open state
        Blocks::WoodenDoor { open, .. }
        | Blocks::IronDoor { open, .. }
        | Blocks::SpruceDoor { open, .. }
        | Blocks::BirchDoor { open, .. }
        | Blocks::JungleDoor { open, .. }
        | Blocks::AcaicaDoor { open, .. }
        | Blocks::DarkOakDoor { open, .. } => {
            if open {
                None // Open door is passable
            } else {
                Some(AABB::new(base_min, base_max))
            }
        }

        // Fence gates - passable when open
        Blocks::FenceGate { open, .. }
        | Blocks::SpruceFenceGate { open, .. }
        | Blocks::BirchFenceGate { open, .. }
        | Blocks::JungleFenceGate { open, .. }
        | Blocks::DarkOakFenceGate { open, .. }
        | Blocks::AcaciaFenceGate { open, .. } => {
            if open {
                None
            } else {
                Some(AABB::new(
                    DVec3::new(base_min.x, base_min.y, base_min.z),
                    DVec3::new(base_max.x, base_min.y + 0.1875, base_max.z),
                ))
            }
        }

        // Soul sand - 0.875 height
        Blocks::SoulSand => Some(AABB::new(
            base_min,
            DVec3::new(base_max.x, base_min.y + 0.875, base_max.z),
        )),

        // Carpets - 0.0625 height
        Blocks::Carpet { .. } => Some(AABB::new(
            base_min,
            DVec3::new(base_max.x, base_min.y + 0.0625, base_max.z),
        )),

        // Snow layers - variable height
        Blocks::SnowLayer { layer_amount } => {
            let height = (layer_amount as f64 + 1.0) / 8.0;
            Some(AABB::new(
                base_min,
                DVec3::new(base_max.x, base_min.y + height, base_max.z),
            ))
        }

        // Pressure plates - 0.0625 height
        Blocks::StonePressurePlate { .. }
        | Blocks::WoodenPressurePlate { .. }
        | Blocks::GoldPressurePlate { .. }
        | Blocks::IronPressurePlate { .. } => Some(AABB::new(
            base_min,
            DVec3::new(base_max.x, base_min.y + 0.0625, base_max.z),
        )),

        // Default: full block
        _ => Some(AABB::new(base_min, base_max)),
    }
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
    Some(AABB::new(
        DVec3::new(base_min.x, half_y.0, base_min.z),
        DVec3::new(base_max.x, half_y.1, base_max.z),
    ))
}

/// Check if a block is passable for projectiles
pub fn is_block_passable(block: Blocks) -> bool {
    get_block_aabb(block, 0, 0, 0).is_none()
}


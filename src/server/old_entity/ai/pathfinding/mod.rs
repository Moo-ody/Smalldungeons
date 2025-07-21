use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::old_entity::ai::pathfinding::check_collision::CollisionType;
use crate::server::old_entity::ai::pathfinding::entity_context::EntityContext;
use crate::server::world::World;

pub mod node;
pub mod pathfinder;
pub mod check_collision;
mod entity_context;

fn get_neighbors(pos: &BlockPos, entity: &EntityContext, world: &World) -> Vec<BlockPos> {
    let mut neighbors = Vec::new();
    let directions = [
        (1, 0), (0, -1),
        (0, 1), (-1, 0),
    ];

    for (dx, dz) in directions {
        let pos = BlockPos {
            x: pos.x + dx,
            y: pos.y,
            z: pos.z + dz,
        };

        if let Some(safe_point) = get_safe_point(pos, entity, world) {
            neighbors.push(safe_point);
        }
    }

    neighbors
}

pub fn is_valid(pos: &BlockPos, entity: &EntityContext, world: &World) -> bool {
    let width = entity.width.ceil() as i32;
    let height = entity.height.ceil() as i32;

    for x in pos.x..pos.x + width {
        for y in pos.y..pos.y + height {
            for z in pos.z..pos.z + width {
                let block = world.get_block_at(x, y, z);
                let path_type = if block == Blocks::Air { CollisionType::Clear } else { CollisionType::Solid };
                if path_type != CollisionType::Clear { // todo rest of this
                    return false;
                }
            }
        }
    }
    true
}

pub fn is_valid_position(pos: &BlockPos, entity: &EntityContext, world: &World) -> bool {
    let below = pos.add_y(-1);
    let block = world.get_block_at(below.x, below.y, below.z);
    let collision = if block == Blocks::Air { CollisionType::Clear } else { CollisionType::Solid };
    let valid = is_valid(pos, entity, world);
    collision == CollisionType::Solid && valid
}

pub fn get_safe_point(pos: BlockPos, entity: &EntityContext, world: &World) -> Option<BlockPos> {
    for dy in 0..=3 { // todo: max fall height.
        let candidate = pos.add_y(-dy);
        if is_valid_position(&candidate, entity, world) {
            return Some(candidate);
        }
    }

    for dy in 1..=1 { //todo: step height
        let candidate = pos.add_y(dy);
        if is_valid_position(&candidate, entity, world) {
            return Some(candidate);
        }
    }

    None
}

pub const fn heuristic(a: &BlockPos, b: &BlockPos) -> i32 {
    a.distance_squared(b)

    // (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs()
}
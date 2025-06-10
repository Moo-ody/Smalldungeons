use crate::server::block::block_pos::BlockPos;
use crate::server::entity::ai::pathfinding::check_collision::CollisionType;
use crate::server::utils::aabb::AABB;
use crate::server::world::World;

pub mod node;
pub mod pathfinder;
pub mod check_collision;

fn get_neighbors(pos: &BlockPos, entity_aabb: &AABB, world: &World) -> Vec<BlockPos> {
    let mut neighbors = Vec::new();
    let directions = [
        (1, 0), (-1, 0),
        (0, 1), (0, -1),
    ];

    for (dx, dz) in directions {
        let pos = BlockPos {
            x: pos.x + dx,
            y: pos.y,
            z: pos.z + dz,
        };

        println!("checking neighbor at {:?}", pos);

        if let Some(safe_point) = get_safe_point(pos, entity_aabb, world) {
            neighbors.push(safe_point);
            println!("neighbor is safe! ")
        } else {
            println!("neighbor is unsafe...")
        }
    }

    neighbors
}

pub fn is_valid(pos: &BlockPos, entity_aabb: &AABB, world: &World) -> bool {
    for x in entity_aabb.min_x.floor() as i32..entity_aabb.max_x.ceil() as i32 {
        for y in entity_aabb.min_y.floor() as i32..entity_aabb.max_y.ceil() as i32 {
            for z in entity_aabb.min_z.floor() as i32..entity_aabb.max_z.ceil() as i32 {
                let block = world.get_block_at(x, y, z);
                let path_type = block.collision_type();
                if path_type != CollisionType::Clear { // todo rest of this
                    return false;
                }
            }
        }
    }
    true
}

pub fn get_safe_point(pos: BlockPos, entity_aabb: &AABB, world: &World) -> Option<BlockPos> {
    let mut target_y = pos.y - 1;
    let collision = world.get_block_at(pos.x, target_y, pos.z).collision_type();
    println!("collision at {:?} is {:?}", pos.replace_y(target_y), collision);

    if collision == CollisionType::Solid && is_valid(&pos, entity_aabb, world) {
        return Some(pos);
    }

    for dy in 1..=3 {
        let ny = pos.y - dy;
        let collision = world.get_block_at(pos.x, ny, pos.z).collision_type();
        if collision == CollisionType::Solid && is_valid(&pos.replace_y(ny), entity_aabb, world) {
            return Some(pos.replace_y(ny));
        }
    }

    for dy in 1..=1 { //todo: step height
        let ny = pos.y + dy;
        let collision = world.get_block_at(pos.x, ny, pos.z).collision_type();
        if collision == CollisionType::Solid && is_valid(&pos.replace_y(ny), entity_aabb, world) {
            return Some(pos.replace_y(ny));
        }
    }

    None
}

pub fn heuristic(a: &BlockPos, b: &BlockPos) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs()
}
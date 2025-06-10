use crate::server::block::block_pos::BlockPos;
use crate::server::entity::ai::pathfinding::node::{NodeData, NodeEntry};
use crate::server::entity::ai::pathfinding::{get_neighbors, heuristic};
use crate::server::entity::entity::Entity;
use crate::server::world::World;
use std::collections::{BinaryHeap, HashMap};

/// Pathfinding reimplementation from minecraft's pathfinding.
/// various things are not ironed out and need accounting for, such as the various types of blocks and potentially their weights.
/// some entity data will need to passed through in some sort of entity context struct, such as aabb (or width/height), step height, max fall height, etc.
pub struct Pathfinder;

impl Pathfinder {
    pub fn new() -> Self {
        Self
    }

    pub fn find_path(entity: &Entity, goal: &BlockPos, world: &World) -> anyhow::Result<Vec<BlockPos>> {
        let mut open = BinaryHeap::new();
        let mut data = HashMap::new();

        let start = BlockPos::from(entity.pos);
        data.insert(start, NodeData { visited: false, tentative_cost: 0.0, parent: None });

        open.push(NodeEntry {
            pos: start,
            total_cost: heuristic(&start, goal) as f32,
        });

        while let Some(NodeEntry { pos: current, .. }) = open.pop() {
            println!("current pos: {:?}", current);
            if current == *goal {
                let mut path = vec![current];
                let mut cur = current;

                while let Some(parent) = data.get(&cur).and_then(|nd| nd.parent) {
                    path.push(parent);
                    cur = parent;
                }

                path.reverse();
                return Ok(path);
            }

            let node_data = data.entry(current).or_insert(NodeData { visited: false, tentative_cost: 0.0, parent: None });
            if node_data.visited {
                continue;
            }
            node_data.visited = true;

            for neighbor_pos in get_neighbors(&current, &entity.positioned_aabb(), world) {
                let tentative = data.get(&current).ok_or_else(|| anyhow::anyhow!("failed to get cost from g_score..."))?.tentative_cost + 1.0; // adjust depending on cost?

                if data.get(&neighbor_pos).map_or(true, |existing| tentative < existing.tentative_cost) {
                    data.insert(neighbor_pos, NodeData {
                        visited: false,
                        tentative_cost: tentative,
                        parent: Some(current),
                    });

                    open.push(NodeEntry {
                        pos: neighbor_pos,
                        total_cost: tentative + heuristic(&neighbor_pos, goal) as f32,
                    });
                }
            }
        }

        // replace with an empty vec when this actually needs to be used.
        Err(anyhow::anyhow!("failed to find path..."))
    }
}
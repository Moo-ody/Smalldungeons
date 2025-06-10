use crate::server::block::block_pos::BlockPos;
use std::cmp::Ordering;

pub struct NodeEntry {
    pub pos: BlockPos,
    pub total_cost: f32,
}

pub struct NodeData {
    pub visited: bool,
    pub tentative_cost: f32,
    pub parent: Option<BlockPos>,
}

impl Ord for NodeEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.total_cost.partial_cmp(&self.total_cost).unwrap()
    }
}

impl PartialOrd for NodeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NodeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.pos == other.pos
    }
}

impl Eq for NodeEntry {}
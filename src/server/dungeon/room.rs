use crate::server::block::block_pos::BlockPos;
use crate::server::world::World;

// for now, only a "prototype" impl
pub struct Room {
    pub position: BlockPos
}

impl Room {

    pub fn load_into_world(&self, world: &mut World) {

    }

    fn tick(world: &World) {

    }
}
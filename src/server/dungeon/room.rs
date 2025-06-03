use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::world::World;

// for now, only a "prototype" impl
pub struct Room {
    pub position: BlockPos
}

impl Room {

    pub fn load_into_world(&self, world: &mut World) {
        for x in self.position.x..self.position.x + 32 {
            for z in self.position.z..self.position.z + 32 {
                world.set_block_at(Blocks::Stone, x, self.position.y, z)
            }
        }
    }

    fn tick(world: &World) {

    }
}
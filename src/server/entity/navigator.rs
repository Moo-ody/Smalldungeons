use crate::server::block::block_pos::BlockPos;
use crate::server::entity::ai::pathfinding::pathfinder::Pathfinder;
use crate::server::entity::entity::Entity;
use crate::server::utils::vec3d::DVec3;
use crate::server::world::World;

pub struct Navigator {
    pub path: Option<Vec<BlockPos>>,
    pub ticks_following: i32,
    pub ticks_at_last_pos: i32,

    pub last_pos: Option<DVec3>,
    // pub path_finder: PathFinder,
}

impl Navigator {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            path: None,
            ticks_following: 0,
            ticks_at_last_pos: 0,

            last_pos: None,
            // path_finder: PathFinder::from_entity(entity),
        }
    }

    pub fn get_path_to_pos(&mut self, entity: &Entity, pos: BlockPos, world: &World) -> anyhow::Result<Vec<BlockPos>> {
        //todo: can navigate
        //todo: search range + 8??

        Pathfinder::find_path(entity, &pos, world)
    }

    pub fn set_path(&mut self, path: Option<Vec<BlockPos>>, entity: &Entity) -> bool {
        if path.is_none() {
            self.path = None;
            return false;
        }

        if path != self.path {
            self.path = path;
        }

        if self.path.as_ref().is_none_or(|path| path.is_empty()) {
            return false;
        }

        self.ticks_at_last_pos = self.ticks_following;
        self.last_pos = Some(entity.pos);
        true
    }
}
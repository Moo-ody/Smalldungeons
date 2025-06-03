use crate::server::block::blocks::Blocks;
use crate::server::chunk::chunk_grid::ChunkGrid;
use crate::server::entity::entity::{Entity, EntityId};
use crate::server::entity::entity_type::EntityType;
use crate::server::server::Server;
use crate::server::utils::vec3f::Vec3f;
use std::collections::HashMap;

pub struct World {
    /// Don't use directly!!, use .server_mut() instead
    /// This is unsafe,
    /// but since server should be alive for the entire program this is fine (I hope)
    pub server: *mut Server,

    pub chunk_grid: ChunkGrid,

    // im thinking of doing something, where
    // a dungeon are always a square (and isn't that big)
    // it could be represented by a flattened 2d array,
    // instead of using a hashmap or now,
    // would allow fast access to a chunk and stuff
    // pub chunks: Vec<Chunk>,

    // entity ids are always positive so they could theoretically be unsigned but minecraft uses signed ints in vanilla and casting might cause weird behavior, also assumes we ever reach the end of i32 though so it might be fine
    pub entities: HashMap<EntityId, Entity>,
    next_entity_id: i32
}

impl World {

    pub fn new() -> World {
        World {
            server: std::ptr::null_mut(),
            chunk_grid: ChunkGrid::new(10),
            entities: HashMap::new(),
            next_entity_id: 1 // might have to start at 1
        }
    }

    pub fn server_mut<'a>(&self) -> &'a mut Server {
        unsafe { self.server.as_mut().unwrap() }
    }

    pub fn new_entity_id(&mut self) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    /// im not sure if functions like these should be here or somewhere else. maybe player impl?
    ///
    /// this can ignore distance if max distance is less than 0.0
    pub fn get_closest_player(&self, pos: &Vec3f, max_distance: f32) -> Option<&Entity> {
        let max_distance_squared = if max_distance > 0.0 { Some(max_distance * max_distance) } else { None };

        // honest i think this looks really bad maybe it should be changed
        self.entities.iter()
            .filter(|(id, e)| {
                e.entity_type == EntityType::Player
            })
            .filter_map(|(id, e)| {
                let distance = e.pos.distance_squared(pos);
                if max_distance_squared.map_or(true, |max_distance_squared| distance < max_distance_squared as f64) {
                    Some((e, distance))
                } else {
                    None
                }
            })
            .min_by(|(_, distance_a), (_, distance_b)| distance_a.partial_cmp(distance_b).unwrap())
            .map(|(e, _)| e)
    }

    pub fn get_closest_in_aabb(&self, aabb: &Vec3f) -> Option<&Entity> {
        None
    }

    pub fn set_block_at(&mut self, block: Blocks, x: i32, y: i32, z: i32) {
        self.chunk_grid.set_block_at(block, x, y, z);
    }

    pub fn get_block_at(&self, x: i32, y: i32, z: i32) -> Blocks {
        self.chunk_grid.get_block_at(x, y, z)
    }
}
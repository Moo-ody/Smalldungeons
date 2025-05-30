use crate::server::chunk::Chunk;
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_type::EntityType;
use crate::server::utils::vec3f::Vec3f;
use std::collections::HashMap;

pub struct World {
    // im thinking of doing something, where
    // a dungeon are always a square (and isn't that big)
    // it could be represented by a flattened 2d array,
    // instead of using a hashmap or now,
    // would allow fast access to a chunk and stuff
    pub chunks: Vec<Chunk>,

    // entity ids are always positive so they could theoretically be unsigned but minecraft uses signed ints in vanilla and casting might cause weird behavior, also assumes we ever reach the end of i32 though so it might be fine
    pub entities: HashMap<i32, Entity>,
    next_entity_id: i32
}

impl World {
    pub fn new() -> World {
        World {
            chunks: Vec::new(),
            entities: HashMap::new(),
            next_entity_id: 1 // might have to start at 1
        }
    }

    pub fn new_entity_id(&mut self) -> i32 {
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
}
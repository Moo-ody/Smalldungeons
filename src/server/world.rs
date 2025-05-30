use crate::server::chunk::Chunk;
use crate::server::entity::entity::Entity;
use std::collections::HashMap;

pub struct World {
    // im thinking of doing something, where
    // a dungeon are always a square (and isn't that big)
    // it could be represented by a flattened 2d array,
    // instead of using a hashmap or now,
    // would allow fast access to a chunk and stuff
    pub chunks: Vec<Chunk>,

    // entity ids are always positive so they could theoretically be unsigned but minecraft uses signed ints in vanilla and casting might cause weird behavior
    pub entities: HashMap<i32, Entity>,
    next_entity_id: i32
}

impl World {
    pub fn new() -> World {
        World {
            chunks: Vec::new(),
            entities: HashMap::new(),
            next_entity_id: 0
        }
    }

    pub fn new_entity_id(&mut self) -> i32 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }
}
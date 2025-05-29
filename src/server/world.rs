use crate::server::chunk::Chunk;
use crate::server::entity::entity_enum::EntityEnum;
use std::collections::HashMap;

pub struct World {
    // im thinking of doing something, where
    // a dungeon are always a square (and isn't that big)
    // it could be represented by a flattened 2d array,
    // instead of using a hashmap or now,
    // would allow fast access to a chunk and stuff
    pub chunks: Vec<Chunk>,
    
    pub entities: HashMap<u32, EntityEnum>,
    next_entity_id: u32
    
}

impl World {
    pub fn new() -> World {
        World {
            chunks: Vec::new(),
            entities: HashMap::new(),
            next_entity_id: 0
        }
    }
    
    pub fn new_entity_id(&mut self) -> u32 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }
}
use std::collections::HashMap;
use std::mem::take;
use crate::server::chunk::Chunk;
use crate::server::entity::entity_enum::{EntityEnum, EntityTrait};

pub struct World {
    next_entity_id: u32,
    pub entities: HashMap<u32, EntityEnum>,
    pub client_to_entities: HashMap<u32, u32>,
    
    pub chunks: Vec<Chunk>,
    pub current_server_tick: u64,
}

impl World {
    pub fn new() -> World {
        World {
            next_entity_id: 0,
            entities: HashMap::new(),
            client_to_entities: HashMap::new(),
            chunks: Vec::new(), 
            current_server_tick: 0
        }
    }
    
    pub fn tick(&mut self) {
        let mut entities = take(&mut self.entities);
        
        for (entity_id, entity) in entities.iter_mut() {
            entity.tick(self).unwrap_or_else(|e|
                eprintln!("Failed to tick {}: {}", entity_id, e.to_string())
            );
        }
        
        self.entities = entities;
    }
    
    pub fn new_entity_id(&mut self) -> u32 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }
    
    pub fn spawn_entity(&mut self, mut entity: EntityEnum) {
        entity.spawn();
        self.entities.insert(entity.get_entity().entity_id, entity);
    }
    
    pub fn remove_entity(&mut self, entity_id: u32) {
        let mut entities = take(&mut self.entities);
        if let Some(entity) = entities.get_mut(&entity_id) {
            entity.despawn(self)
        }
        self.entities = entities;
        self.entities.remove(&entity_id);
    }
    
    
    pub fn get_player_from_client_id(&mut self, client_id: u32) -> Option<&mut EntityEnum> {
        self.client_to_entities.get(&client_id).and_then(|id| self.entities.get_mut(id))
    }
    
    pub fn remove_player_from_client_id(&mut self, client_id: &u32) {
        if let Some(id) = self.client_to_entities.remove(client_id) {
            self.entities.remove(&id);
        }
    }
}
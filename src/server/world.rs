use std::collections::HashMap;
use std::mem::take;
use tokio::sync::mpsc::UnboundedSender;
use crate::net::network_message::NetworkMessage;
use crate::server::chunk::Chunk;
use crate::server::entity::entity_enum::{EntityEnum, EntityTrait};
use crate::server::utils::vec3f::Vec3f;

/// this is used to store all data including server right now, but a server can have multiple worlds. 
/// it doesnt matter right now (or potentially ever given the project) since we only need one world,
/// itd be best to support multiple worlds probably.
/// 
/// world data (chunks, world spawn, entities, etc.) should be moved to a new struct and this renamed to server.
/// Entities should include a reference to the world id, or at least some way to quickly get the world theyre a part of.
pub struct World {
    pub network_tx: UnboundedSender<NetworkMessage>,
    
    next_entity_id: u32,
    pub entities: HashMap<u32, EntityEnum>,
    pub client_to_entities: HashMap<u32, u32>,
    
    pub chunks: Vec<Chunk>,
    pub current_server_tick: u64,
    pub world_spawn: Vec3f,
}

impl World {
    pub fn with_net_tx(network_tx: UnboundedSender<NetworkMessage>) -> World {
        World {
            network_tx,
            next_entity_id: 0,
            entities: HashMap::new(),
            client_to_entities: HashMap::new(),
            chunks: Vec::new(),
            current_server_tick: 0,
            world_spawn: Vec3f::new_empty()
        }
    }
    
    pub fn tick(&mut self) {
        let mut entities = take(&mut self.entities);
        
        for (entity_id, entity) in entities.iter_mut() {
            entity.get_entity().ticks_existed += 1;
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
            println!("Removing player with id {}", id);
            self.entities.remove(&id);
        }
    }
}
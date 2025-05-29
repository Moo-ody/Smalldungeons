use crate::net::network_message::NetworkMessage;
use crate::server::chunk::Chunk;
use crate::server::entity::entity_enum::EntityEnum::PlayerEntity;
use crate::server::entity::entity_enum::{EntityEnum, EntityTrait};
use crate::server::entity::player_entity;
use crate::server::utils::vec3f::Vec3f;
use anyhow::bail;
use std::collections::HashMap;
use std::mem::take;
use tokio::sync::mpsc::UnboundedSender;

/// This is used to store all data including server right now, but a server can have multiple worlds.
/// It doesnt matter right now (or potentially ever given the project) since we only need one world,
/// itd be best to support multiple worlds probably.
///
/// World data (chunks, world spawn, entities, etc.) should be moved to a new struct and this renamed to server.
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
            world_spawn: Vec3f::new_empty(),
        }
    }
    
    pub fn tick(&mut self) {
        self.current_server_tick += 1;

        // this weirdness is done so we can manipulate other entities in any given entity's tick.
        // it should be refactored if we end up not needing to.
        // keys can be cloned since if the entity is removed by another before its tick, it will return None and get skipped.
        // this will not tick a spawned entity on its first tick if its spawned by another entity though so well see
        for entity_id in self.entities.keys().cloned().collect::<Vec<_>>() {
            if let Some(mut entity) = self.entities.remove(&entity_id) {
                entity.get_entity().ticks_existed += 1;
                let returned = entity.tick(self);
                self.entities.insert(entity_id, returned);
            }
        }
    }
    
    pub fn new_entity_id(&mut self) -> u32 {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }
    
    pub fn spawn_entity(&mut self, mut entity: EntityEnum) {
        entity.spawn(self);
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


    pub fn get_player_from_client_id(&mut self, client_id: u32) -> anyhow::Result<&mut player_entity::PlayerEntity> {
        let entity_id = self.client_to_entities.get(&client_id).ok_or_else(|| anyhow::anyhow!("Client id doesnt have a corresponding entity."))?;
        let entity_enum = self.entities.get_mut(entity_id).ok_or_else(|| anyhow::anyhow!("Entity id doesnt have a corresponding entity."))?;
        if let PlayerEntity(player) = entity_enum {
            Ok(player)
        } else {
            bail!("Entity is not a player")
        }
    }

    pub fn remove_player_from_client_id(&mut self, client_id: &u32) -> anyhow::Result<()> {
        if let Some(id) = self.client_to_entities.remove(client_id) {
            println!("Removing player with id {}", id);
            if self.entities.remove(&id).is_none() {
                bail!("Player entity not found")
            }
            Ok(())
        } else {
            bail!("Player not found")
        }
    }
}
use crate::server::entity::entity_enum::Entity;

pub struct World {
    pub entities: Vec<Entity>
}

impl World {
    pub fn new() -> World {
        World {
            entities: Vec::new()
        }
    }
    
    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }
}
use crate::server::entity::entity::Entity;

pub struct EntityContext {
    pub width: f32,
    pub height: f32,
}

impl EntityContext {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            width: entity.width,
            height: entity.height,
        }
    }
}

use crate::server::entity::entity::Entity;
use crate::server::entity::entity_enum::{EntityEnum, EntityTrait};
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;

pub struct PlayerEntity {
    pub client_id: u32,
    pub entity: Entity,
}

impl PlayerEntity {
    pub fn spawn_at(pos: Vec3f, id: u32, world: &mut World) -> PlayerEntity {
        PlayerEntity {
            client_id: id,
            entity: Entity::spawn_at(pos, world.new_entity_id())
        }
    }
}

impl EntityTrait for PlayerEntity {
    
    fn get_entity(&mut self) -> &mut Entity {
        &mut self.entity
    }

    fn tick(&mut self, world: &mut World) -> anyhow::Result<()> {
        // confirm transaction packet for mods goes here
        
        if self.client_id != 0 {
            // keep alive logic probably goes here in some way. world has current tick for timing.
        }
        Ok(())
    }

    fn spawn(&mut self)  {
        // todo
    }

    fn despawn(&mut self, world: &mut World) {
        // todo
    }
}
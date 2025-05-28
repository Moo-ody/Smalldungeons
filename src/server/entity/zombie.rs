use crate::server::entity::entity::Entity;
use crate::server::entity::entity_enum::EntityTrait;
use crate::server::entity::metadata::{Metadata, MetadataEntry, MetadataImpl};
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;
use crate::{meta_data, meta_data_impl};

#[derive(Debug)]
pub struct Zombie {
    entity: Entity,
    is_child: IsChild,
    is_villager: IsVillager,
    is_converting: IsConverting
}

meta_data!(IsChild, bool, 12);
meta_data!(IsVillager, bool, 13);
meta_data!(IsConverting, bool, 14);
meta_data_impl!(Zombie, is_child, is_villager, is_converting);

impl Zombie {
    pub fn create_at(pos: Vec3f, world: &mut World) -> Zombie {
        Zombie {
            entity: Entity::spawn_at(pos, world.new_entity_id()),
            is_child: IsChild(true),
            is_villager: IsVillager(false),
            is_converting: IsConverting(false)
        }
    }
}

impl EntityTrait for Zombie {
    fn get_id(&self) -> i8 {
        54
    }

    fn get_entity(&mut self) -> &mut Entity {
        &mut self.entity
    }

    fn tick(mut self, world: &mut World) -> Self {
        // todo
        self
    }

    fn spawn(&mut self, world: &mut World) {
        // todo!()
    }

    fn despawn(&mut self, world: &mut World) {
        // todo!()
    }
}
use enum_dispatch::enum_dispatch;
use crate::server::entity::entity::Entity;
use crate::server::entity::player_entity::PlayerEntity;
use crate::server::entity::zombie::Zombie;
use crate::server::world::World;

#[enum_dispatch(EntityTrait)]
pub enum EntityEnum {
    PlayerEntity(PlayerEntity),
    ZombieEntity(Zombie)
}

#[enum_dispatch]
pub trait EntityTrait {
    fn get_id(&self) -> i8;
    fn get_entity(&mut self) -> &mut Entity;
    fn tick(&mut self, world: &mut World) -> anyhow::Result<()>;
    fn spawn(&mut self);
    fn despawn(&mut self, world: &mut World);
}
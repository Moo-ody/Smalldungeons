use crate::server::entity::entity::Entity;
use crate::server::entity::player_entity::PlayerEntity;
use crate::server::entity::zombie::Zombie;
use crate::server::old_world::World;
use enum_dispatch::enum_dispatch;

// idk how I feel about this,
// like enum that leads into a struct,
// i think we have a struct with common
// with an enum to just represent like mob type, like zombie, stuff
// or even instead just represent custom mobs 
// and then handle type with packets inside a function or something,
// since we shouldnt need to much "vanilla interface"

#[enum_dispatch(EntityTrait)]
#[derive(Debug)]
pub enum EntityEnum {
    PlayerEntity(PlayerEntity),
    ZombieEntity(Zombie)
}

#[enum_dispatch]
pub trait EntityTrait
where
    Self: Sized,
{
    fn get_id(&self) -> i8;
    fn get_entity(&mut self) -> &mut Entity;
    fn tick(mut self, world: &mut World) -> Self {
        self
    }
    fn spawn(&mut self, world: &mut World);
    fn despawn(&mut self, world: &mut World);
}
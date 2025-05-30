use rand::{random, random_range};
use crate::id_enum;
use crate::server::entity::entity_type::EntityType;
use crate::server::world::World;

/// entity enum macro system can be easily implemented for this too and would remove the giant match statements and such. (well they wouldnt be removed, just hidden in the macro)
id_enum! {
    pub enum TaskType: u8 {
        WatchClosest (1), // todo: get actual bitmask values
        AttackOnCollide (2),
        MoveToBlock (3),
        Panic (4),
    }
}

impl TaskType {
    pub const fn is_compatible(&self, other: &Self) -> bool {
        (self.id() & other.id()) == 0
    }
}
pub const ID: i8 = 368;
pub const WIDTH: f32 = 0.25;
pub const HEIGHT: f32 = 0.25;

use crate::server::old_entity::metadata::EntityMetadata;
use crate::server::old_entity::ai::ai_tasks::AiTasks;

pub fn ai_tasks() -> Option<AiTasks> {
    None
}

pub fn metadata() -> EntityMetadata {
    EntityMetadata::new()
}
impl EntityImpl for PearlEntityImpl {
    fn spawn(&mut self, entity: &mut Entity) {
        // Tell clients the initial motion so the pearl animates client-side
        for player in entity.world_mut().players.values() {
            let _ = player.send_packet(EntityVelocity {
                entity_id: entity.id,
                motion_x: self.velocity.x,
                motion_y: self.velocity.y,
                motion_z: self.velocity.z,
            });
        }
    }

    fn tick(&mut self, entity: &mut Entity) {
        // (keep your physics/collision the same)
        // ...
    }
}

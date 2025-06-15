use crate::server::entity::entity::Entity;
use crate::server::utils::vec3f::Vec3f;

/// impl for the movement of entities to keep entity file clean.
impl Entity {
    pub fn set_move(&mut self, speed: f32) {
        self.entity_move_data.move_forward = speed
    }

    // pub fn move_entity(&mut self, mut new_pos: Vec3f) {
    //     let mut flag: bool;
    //     let mut temp_pos = self.pos;
    //     // if in web
    //     
    //     flag = false //self.on_ground // sneaking check
    //     
    //     if flag {
    //         
    //     }
    // }
}
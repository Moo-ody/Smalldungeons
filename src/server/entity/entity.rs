use crate::server::utils::aabb::AABB;
use crate::server::utils::vec3f::Vec3f;

#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_id: u32,
    pub pos: Vec3f,
    pub motion: Vec3f,
    pub prev_pos: Vec3f,
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub aabb: AABB,
    pub is_dead: bool,
    pub height: f32,
    pub width: f32,
    pub ticks_existed: u32,
}

impl Entity {
    pub fn spawn_at(pos: Vec3f, id: u32) -> Entity {
        Entity {
            entity_id: id,
            pos: pos.clone(),
            motion: Vec3f::new_empty(),
            prev_pos: pos.clone(),
            yaw: 0.0,
            pitch: 0.0,
            head_yaw: 0.0,
            aabb: AABB::new_empty(),
            is_dead: false,
            height: 0.0,
            width: 0.0,
            ticks_existed: 0,       
        }
    }
    
    pub fn update_position(&mut self, x: f64, y: f64, z: f64) {
        self.prev_pos = self.pos.clone();
        self.pos.x = x;
        self.pos.y = y;
        self.pos.z = z;
    }
}
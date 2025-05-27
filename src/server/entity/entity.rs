
use crate::server::utils::axis_aligned_bounding_box::AxisAlignedBoundingBox;
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
    pub axis_aligned_bb: AxisAlignedBoundingBox,
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
            yaw: 0f32,
            pitch: 0f32,
            head_yaw: 0f32,
            axis_aligned_bb: AxisAlignedBoundingBox::new_empty(),
            is_dead: false,
            height: 0f32,
            width: 0f32,
            ticks_existed: 0u32,       
        }
    }
    
    pub fn update_position(&mut self, x: f64, y: f64, z: f64) {
        self.prev_pos = self.pos.clone();
        self.pos.x = x;
        self.pos.y = y;
        self.pos.z = z;
    }
}
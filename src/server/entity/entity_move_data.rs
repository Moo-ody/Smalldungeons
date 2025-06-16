use crate::server::utils::vec3f::Vec3f;

#[derive(Clone, Debug)]
pub struct EntityMoveData {
    pub new_pos: Vec3f,
    pub new_yaw: f32,
    pub new_pitch: f32,

    pub move_forward: f32,
    pub move_strafe: f32,
}

impl EntityMoveData {
    pub fn new() -> Self {
        Self {
            new_pos: Vec3f::new_empty(),
            new_yaw: 0.0,
            new_pitch: 0.0,

            move_forward: 0.0,
            move_strafe: 0.0,
        }
    }
}
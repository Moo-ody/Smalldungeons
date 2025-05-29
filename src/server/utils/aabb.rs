use crate::server::utils::vec3f::Vec3f;

#[derive(Debug, Clone)]
pub struct AABB {
    pub bottom_corner: Vec3f,
    pub top_corner: Vec3f,
}

impl AABB {
    pub fn new_empty() -> AABB {
        AABB {
            bottom_corner: Vec3f::new_empty(),
            top_corner: Vec3f::new_empty(),
        }
    }
}
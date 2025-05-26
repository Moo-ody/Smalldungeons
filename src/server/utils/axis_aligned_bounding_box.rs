use crate::server::utils::vec3f::Vec3f;

pub struct AxisAlignedBoundingBox {
    pub bottom_corner: Vec3f,
    pub top_corner: Vec3f,
}

impl AxisAlignedBoundingBox {
    pub fn new_empty() -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox {
            bottom_corner: Vec3f::new_empty(),
            top_corner: Vec3f::new_empty(),
        }
    }
}
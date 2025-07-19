use crate::server::utils::dvec3::DVec3;

#[derive(Debug, Clone)]
pub struct AABB {
    pub min: DVec3,
    pub max: DVec3,
}

impl AABB {

    pub const ZERO: AABB = AABB {
        min: DVec3::ZERO,
        max: DVec3::ZERO,
    };

    pub fn new(min: DVec3, max: DVec3) -> Self {
        Self {
            min,
            max,
        }
    }

    pub const fn from_height_width(height: f64, width: f64) -> Self {
        Self { 
            min: DVec3 { x: -width / 2.0, y: 0.0, z: -width / 2.0 },
            max: DVec3 { x: width / 2.0, y: height, z: width / 2.0 }
        }
    }
}
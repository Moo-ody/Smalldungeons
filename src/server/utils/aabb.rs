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
        Self { min, max }
    }
    
    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }

    /// Create an AABB centered at origin with given width/height.
    pub const fn from_height_width(height: f64, width: f64) -> Self {
        Self { 
            min: DVec3 { x: -width / 2.0, y: 0.0, z: -width / 2.0 },
            max: DVec3 { x:  width / 2.0, y: height, z:  width / 2.0 }
        }
    }

    /// Create an axis-aligned box with given width/height at origin.
    pub const fn from_width_height(width: f64, height: f64) -> Self {
        Self {
            min: DVec3 { x: 0.0, y: 0.0, z: 0.0 },
            max: DVec3 { x: width, y: height, z: width },
        }
    }

    /// Offset this AABB by a vector, returning a new box.
    pub fn offset(self, dvec3: DVec3) -> AABB {
        AABB::new(self.min + dvec3, self.max + dvec3)
    }
}
#[derive(Debug, Clone)]
pub struct AABB {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

impl AABB {
    pub fn new_empty() -> AABB {
        AABB {
            min_x: 0.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 0.0,
            max_y: 0.0,
            max_z: 0.0,
        }
    }

    pub fn from_height_width(height: f64, width: f64) -> AABB {
        AABB {
            min_x: 0.0 - width / 2.0, //idrk if this is right but whatever
            min_y: 0.0,
            min_z: 0.0 - width / 2.0,
            max_x: 0.0 + width / 2.0,
            max_y: height,
            max_z: 0.0 + width / 2.0,
        }
    }
}
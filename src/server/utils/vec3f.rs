#[derive(Clone)]
pub struct Vec3f {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3f {
    pub fn new_empty() -> Vec3f {
        Vec3f { x: 0.0, y: 0.0, z: 0.0 }
    }
}
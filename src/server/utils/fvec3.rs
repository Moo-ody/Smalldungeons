use crate::server::utils::dvec3::DVec3;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct FVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl FVec3 {
    
    pub const ZERO: FVec3 = FVec3 { 
        x: 0.0,
        y: 0.0, 
        z: 0.0 
    };
    
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl From<DVec3> for FVec3 {
    fn from(value: DVec3) -> Self {
        Self {
            x: value.x as f32,
            y: value.y as f32,
            z: value.z as f32,
        }
    }
}
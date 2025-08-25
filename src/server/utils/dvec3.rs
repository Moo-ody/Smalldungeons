use crate::server::block::block_position::BlockPos;
use crate::server::utils::direction::Direction;
use std::ops::{Add, Div, Mul, Sub};

/// Double (f64) Vec3
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct DVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl DVec3 {

    pub const ZERO: DVec3 = DVec3 {
        x: 0.0, 
        y: 0.0,
        z: 0.0
    };

    pub fn new(x: f64, y: f64, z: f64) -> DVec3 {
        DVec3 {
            x,
            y,
            z,
        }
    }
    
    pub fn from_centered(block_pos: &BlockPos) -> Self {
        Self::new(block_pos.x as f64 + 0.5, block_pos.y as f64 + 0.5, block_pos.z as f64 + 0.5)
    }

    pub fn from_x(x: f64) -> DVec3 {
        DVec3 {
            x,
            y: 0.0,
            z: 0.0,
        }
    }
    
    pub fn from_y(y: f64) -> DVec3 {
        DVec3 {
            x: 0.0,
            y,
            z: 0.0,
        }
    }
    
    pub fn from_z(z: f64) -> DVec3 {
        DVec3 {
            x: 0.0,
            y: 0.0,
            z,
        }
    }

    pub fn add_x(&mut self, amount: f64) -> Self {
        self.x += amount;
        *self
    }

    pub fn add_y(&mut self, amount: f64) -> Self {
        self.y += amount;
        *self
    }

    pub fn add_z(&mut self, amount: f64) -> Self {
        self.z += amount;
        *self
    }

    pub fn normalize(&self) -> DVec3 {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if len < 1.0e-4 {
            DVec3 { x: 0.0, y: 0.0, z: 0.0 }
        } else {
            DVec3 { x: self.x / len, y: self.y / len, z: self.z / len }
        }
    }

    pub fn rotate(&self, rotation: Direction) -> Self {
        match rotation {
            Direction::North => Self { x: self.x, y: self.y, z: self.z },
            Direction::East => Self { x: -self.z, y: self.y, z: self.x },
            Direction::South => Self { x: -self.x, y: self.y, z: -self.z },
            Direction::West => Self { x: self.z, y: self.y, z: -self.x },
            _ => Self { x: self.x, y: self.y, z: self.z },
        }
    }

    pub fn distance_to(&self, other: &DVec3) -> f64 {
        self.distance_squared(other).sqrt()
    }

    pub fn distance_squared(&self, other: &DVec3) -> f64 {
        let x = self.x - other.x;
        let y = self.y - other.y;
        let z = self.z - other.z;
        z.mul_add(z, x.mul_add(x, y * y))
    }
}

impl Add for DVec3 {
    type Output = DVec3;
    
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for DVec3 {
    type Output = DVec3;

    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Div for DVec3 {
    type Output = DVec3;
    
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        }
    }
}

impl Mul for DVec3 {
    type Output = DVec3;
    
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl From<&BlockPos> for DVec3 {
    fn from(pos: &BlockPos) -> Self {
        Self::new(pos.x as f64, pos.y as f64, pos.z as f64)
    }
}
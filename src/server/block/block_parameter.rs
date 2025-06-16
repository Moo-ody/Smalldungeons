use crate::server::utils::direction::{self, Direction};



#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Axis {
    Y, // y is first for whatever reason
    X,
    Z,
    None
}

impl Axis {
    pub fn rotate(&self, direction: Direction) -> Axis {
        match self {
            Axis::X => match direction {
                Direction::South | Direction::North => Axis::Z,
                _ => Axis::X,
            },
            Axis::Z => match direction {
                Direction::East | Direction::West => Axis::X,
                _ => Axis::Z,
            },
            Axis::Y => Axis::Y,
            Axis::None => Axis::None,
        }
    }
}
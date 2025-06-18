use crate::server::block::metadata::BlockMetadata;
use crate::server::utils::direction::Direction;


#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone, Eq)]
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

impl BlockMetadata for Axis {
    fn meta_size() -> u8 {
        2
    }

    fn get_meta(&self) -> u8 {
        *self as u8
    }

    fn from_meta(meta: u8) -> Self {
        match meta & 0b11 {
            0 => Axis::Y,
            1 => Axis::X,
            2 => Axis::Z,
            _ => Axis::None,
        }
    }
}

// TODO, ROTATABLE
#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone, Eq)]
pub enum LeverOrientation {
    DownX,
    East,
    West,
    South,
    North,
    UpZ,
    UpX,
    DownZ
}

impl BlockMetadata for LeverOrientation {
    fn meta_size() -> u8 {
        3
    }

    fn get_meta(&self) -> u8 {
        *self as u8
    }

    fn from_meta(meta: u8) -> Self {
        match meta & 0x7 {
            0 => LeverOrientation::DownX,
            1 => LeverOrientation::East,
            2 => LeverOrientation::West,
            3 => LeverOrientation::South,
            4 => LeverOrientation::North,
            5 => LeverOrientation::UpZ,
            6 => LeverOrientation::UpX,
            _ => LeverOrientation::DownZ,
        }
    }
}

// why couldn't mojang re-use their code ever???
// this is essentially same as HorizontalDirection,
// but slightly reordered
#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone, Eq)]
pub enum TrapdoorDirection {
    North,
    South,
    West,
    East
}

impl BlockMetadata for TrapdoorDirection {
    fn meta_size() -> u8 {
        2
    }

    fn get_meta(&self) -> u8 {
        *self as u8
    }

    fn from_meta(meta: u8) -> Self {
        match meta & 0b11 {
            0 => TrapdoorDirection::North,
            1 => TrapdoorDirection::South,
            2 => TrapdoorDirection::West,
            _ => TrapdoorDirection::East
        }
    }
}

// needs to be here so it can rotate
#[repr(transparent)]
#[derive(PartialEq, Debug, Copy, Clone, Eq)]
pub struct VineMetadata(u8);

impl VineMetadata {
    pub fn meta_size() -> u8 {
        4
    }
    pub fn get_meta(&self) -> u8 {
        self.0
    }
    pub fn from_meta(meta: u8) -> Self {
        VineMetadata(meta)
    }
}


// pmo

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HayAxis {
    Y = 0,
    X = 4,
    Z = 8,
    None = 12
}

impl BlockMetadata for HayAxis {
    fn meta_size() -> u8 {
        4
    }
    fn get_meta(&self) -> u8 {
        *self as u8
    }
    fn from_meta(meta: u8) -> Self {
        match meta & 0b1100 {
            0 => HayAxis::Y,
            4 => HayAxis::X,
            8 => HayAxis::Z,
            _ => HayAxis::None,
        }
    }
}
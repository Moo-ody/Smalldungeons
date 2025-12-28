use crate::server::block::metadata::BlockMetadata;
use crate::server::block::rotatable::Rotatable;
use crate::server::utils::direction::Direction;
use blocks::BlockMetadata;

/// This type of rotation is used in blocks like Logs, etc
// TODO: This needs Correct rotation
#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone, Eq, BlockMetadata)]
pub enum Axis {
    Y,
    X,
    Z,
    None
}

impl Rotatable for Axis {
    fn rotate(&self, other: Direction) -> Self {
        match other {
            Direction::North | Direction::South => {
                *self
            }
            Direction::East | Direction::West => {
                match self {
                    Axis::Y => Axis::Y,
                    Axis::X => Axis::Z,
                    Axis::Z => Axis::X,
                    Axis::None => Axis::None,
                }
            }
            _ => unreachable!()
        }
    }
}

impl Axis {
    pub fn get_direction(&self) -> Direction {
        match self {
            Axis::X => Direction::East,
            Axis::Z => Direction::North,
            _ => Direction::Up
        }
    }
}

/// Used for exclusively lever.
#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone, Eq, BlockMetadata)]
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

impl Rotatable for LeverOrientation {
    fn rotate(&self, other: Direction) -> Self {
        match other {
            Direction::North => *self, // No rotation
            Direction::East => {
                match self {
                    LeverOrientation::North => LeverOrientation::East,
                    LeverOrientation::East => LeverOrientation::South,
                    LeverOrientation::South => LeverOrientation::West,
                    LeverOrientation::West => LeverOrientation::North,
                    LeverOrientation::DownX => LeverOrientation::DownZ,
                    LeverOrientation::DownZ => LeverOrientation::DownX,
                    LeverOrientation::UpX => LeverOrientation::UpZ,
                    LeverOrientation::UpZ => LeverOrientation::UpX,
                }
            }
            Direction::South => {
                match self {
                    LeverOrientation::North => LeverOrientation::South,
                    LeverOrientation::East => LeverOrientation::West,
                    LeverOrientation::South => LeverOrientation::North,
                    LeverOrientation::West => LeverOrientation::East,
                    LeverOrientation::DownX => LeverOrientation::DownX,
                    LeverOrientation::DownZ => LeverOrientation::DownZ,
                    LeverOrientation::UpX => LeverOrientation::UpX,
                    LeverOrientation::UpZ => LeverOrientation::UpZ,
                }
            }
            Direction::West => {
                match self {
                    LeverOrientation::North => LeverOrientation::West,
                    LeverOrientation::East => LeverOrientation::North,
                    LeverOrientation::South => LeverOrientation::East,
                    LeverOrientation::West => LeverOrientation::South,
                    LeverOrientation::DownX => LeverOrientation::DownZ,
                    LeverOrientation::DownZ => LeverOrientation::DownX,
                    LeverOrientation::UpX => LeverOrientation::UpZ,
                    LeverOrientation::UpZ => LeverOrientation::UpX,
                }
            }
            _ => *self, // Up/Down don't affect horizontal rotation
        }
    }
}

// TODO: Rotate, or maybe wrap around a different direction and just have different Blockmetadata impl
#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone, Eq, BlockMetadata)]
pub enum TrapdoorDirection {
    North,
    South,
    West,
    East
}

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

impl Rotatable for VineMetadata {
    fn rotate(&self, other: Direction) -> Self {
        let meta = self.0;
        match other {
            Direction::North => *self, // No rotation
            Direction::East => {
                // Rotate 90° clockwise: North→East, East→South, South→West, West→North
                let mut new_meta = 0u8;
                if meta & 0x1 != 0 { new_meta |= 0x4; } // North → East
                if meta & 0x4 != 0 { new_meta |= 0x2; } // East → South
                if meta & 0x2 != 0 { new_meta |= 0x8; } // South → West
                if meta & 0x8 != 0 { new_meta |= 0x1; } // West → North
                VineMetadata(new_meta)
            }
            Direction::South => {
                // Rotate 180°: North↔South, East↔West
                let mut new_meta = 0u8;
                if meta & 0x1 != 0 { new_meta |= 0x2; } // North → South
                if meta & 0x2 != 0 { new_meta |= 0x1; } // South → North
                if meta & 0x4 != 0 { new_meta |= 0x8; } // East → West
                if meta & 0x8 != 0 { new_meta |= 0x4; } // West → East
                VineMetadata(new_meta)
            }
            Direction::West => {
                // Rotate 90° counter-clockwise: North→West, West→South, South→East, East→North
                let mut new_meta = 0u8;
                if meta & 0x1 != 0 { new_meta |= 0x8; } // North → West
                if meta & 0x8 != 0 { new_meta |= 0x2; } // West → South
                if meta & 0x2 != 0 { new_meta |= 0x4; } // South → East
                if meta & 0x4 != 0 { new_meta |= 0x1; } // East → North
                VineMetadata(new_meta)
            }
            _ => *self, // Up/Down don't affect horizontal rotation
        }
    }
}

/// custom direction for Torch blocks, since they used a slightly modified version of direction.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, BlockMetadata)]
pub enum TorchDirection {
    East = 1,
    West = 2,
    South = 3,
    North = 4,
    Up = 5,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, BlockMetadata)]
pub enum HorizontalDirection {
    South,
    West,
    North,
    East,
}

impl Rotatable for HorizontalDirection {
    fn rotate(&self, other: Direction) -> Self {
        match other {
            Direction::North => {
                match self {
                    HorizontalDirection::North => HorizontalDirection::North,
                    HorizontalDirection::East => HorizontalDirection::East,
                    HorizontalDirection::South => HorizontalDirection::South,
                    HorizontalDirection::West => HorizontalDirection::West,
                }
            },
            Direction::East => {
                match self {
                    HorizontalDirection::North => HorizontalDirection::East,
                    HorizontalDirection::East => HorizontalDirection::South,
                    HorizontalDirection::South => HorizontalDirection::West,
                    HorizontalDirection::West => HorizontalDirection::North,
                }
            }
            Direction::South => {
                match self {
                    HorizontalDirection::North => HorizontalDirection::South,
                    HorizontalDirection::East => HorizontalDirection::West,
                    HorizontalDirection::South => HorizontalDirection::North,
                    HorizontalDirection::West => HorizontalDirection::East,
                }
            }
            Direction::West => {
                match self {
                    HorizontalDirection::North => HorizontalDirection::West,
                    HorizontalDirection::East => HorizontalDirection::North,
                    HorizontalDirection::South => HorizontalDirection::East,
                    HorizontalDirection::West => HorizontalDirection::South,
                }
            }
            _ => unreachable!()

        }
    }
}


#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, BlockMetadata)]
pub enum StairDirection {
    East,
    West,
    South,
    North,
}

impl Rotatable for StairDirection {
    fn rotate(&self, other: Direction) -> Self {
        match other {
            Direction::North => {
                match self {
                    StairDirection::North => StairDirection::North,
                    StairDirection::East => StairDirection::East,
                    StairDirection::South => StairDirection::South,
                    StairDirection::West => StairDirection::West,
                }
            },
            Direction::East => {
                match self {
                    StairDirection::North => StairDirection::East,
                    StairDirection::East => StairDirection::South,
                    StairDirection::South => StairDirection::West,
                    StairDirection::West => StairDirection::North,
                }
            }
            Direction::South => {
                match self {
                    StairDirection::North => StairDirection::South,
                    StairDirection::East => StairDirection::West,
                    StairDirection::South => StairDirection::North,
                    StairDirection::West => StairDirection::East,
                }
            }
            Direction::West => {
                match self {
                    StairDirection::North => StairDirection::West,
                    StairDirection::East => StairDirection::North,
                    StairDirection::South => StairDirection::East,
                    StairDirection::West => StairDirection::South,
                }
            }
            _ => unreachable!()

        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonDirection(Direction);


// todo: fix rotation, its still broken even with fixed direction values
impl Rotatable for ButtonDirection {
    fn rotate(&self, direction: Direction) -> Self {
        ButtonDirection(self.0.rotate(direction))
    }
}

impl BlockMetadata for ButtonDirection {
    fn meta_size() -> u8 {
        3
    }
    fn get_meta(&self) -> u8 {
        match self.0 {
            Direction::Down => 0,
            Direction::East => 1,
            Direction::West => 2,
            Direction::South => 3,
            Direction::North => 4,
            Direction::Up => 5,
        }
    }
    fn from_meta(meta: u8) -> Self {
        ButtonDirection(match meta & 0b111 {
            0 => Direction::Down,
            1 => Direction::East,
            2 => Direction::West,
            3 => Direction::South,
            4 => Direction::North,
            _ => Direction::Up, // 5–7 fall back to Up
        })
    }
}
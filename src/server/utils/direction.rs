use crate::server::block::metadata::BlockMetadata;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    North, // -Z
    East, // // +X
    South, // +Z
    West, // -X
    Up,
    Down,
}

impl BlockMetadata for Direction {
    fn meta_size() -> u8 {
        2
    }
    fn get_meta(&self) -> u8 {
        *self as u8
    }

    fn from_meta(meta: u8) -> Self {
        match meta {
            0 => Direction::Down,
            1 => Direction::Up,
            2 => Direction::North,
            3 => Direction::South,
            4 => Direction::West,
            _ => Direction::East,
        }
    }
}

impl Direction {
    
    // example
    pub fn rotate(&self) {
        println!("rotated direction {:?}", self);
    }
    
    /// Index is just the degrees / 90
    /// 
    /// Eg 0 = 0 degrees, 1 = 90 deg, 2 = 180 deg, 3 = 270 deg
    pub fn rotate_by_index(&self, index: usize) -> Direction {
        match index % 4 {
            0 => {
                match self {
                    Direction::North => Direction::North,
                    Direction::East => Direction::East,
                    Direction::South => Direction::South,
                    Direction::West => Direction::West,
                    Direction::Up => Direction::Up,
                    Direction::Down => Direction::Down,
                }
            },
            1 => {
                match self {
                    Direction::North => Direction::East,
                    Direction::East => Direction::South,
                    Direction::South => Direction::West,
                    Direction::West => Direction::North,
                    Direction::Up => Direction::Up,
                    Direction::Down => Direction::Down,
                }
            }
            2 => {
                match self {
                    Direction::North => Direction::South,
                    Direction::East => Direction::West,
                    Direction::South => Direction::North,
                    Direction::West => Direction::East,
                    Direction::Up => Direction::Up,
                    Direction::Down => Direction::Down,
                }
            }
            3 => {
                match self {
                    Direction::North => Direction::West,
                    Direction::East => Direction::North,
                    Direction::South => Direction::East,
                    Direction::West => Direction::South,
                    Direction::Up => Direction::Up,
                    Direction::Down => Direction::Down,
                }
            }
            _ => unreachable!()
               
        }
    }

    pub fn get_offset(&self) -> (i32, i32, i32) {
        match self {
            Direction::North => (0, 0, -1),
            Direction::East => (1, 0, 0),
            Direction::South => (0, 0, 1),
            Direction::West => (-1, 0, 0),
            Direction::Up => (0, 1, 0),
            Direction::Down => (0, -1, 0),
        }
    }

    /// The stair index is the metadata for which direction the stair is facing. At meta=0, the stair is facing East etc.
    pub fn get_stair_index(&self) -> u8 {
        match self {
            Direction::East => 0,
            Direction::West => 1,
            Direction::South => 2,
            Direction::North => 3,
            _ => 0,
        }
    }

    pub fn get_piston_index(&self) -> u8 {
        match self {
            Direction::Down => 0,
            Direction::Up => 1,
            Direction::North => 2,
            Direction::South => 3,
            Direction::West => 4,
            Direction::East => 5,
        }
    }
    
    pub fn get_torch_meta(&self) -> u8 {
        match self {
            Direction::West => 1,
            Direction::South => 2,
            Direction::North => 3,
            Direction::East => 4,
            _ => 0,
        }
        
    }
}
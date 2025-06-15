
#[repr(u8)]
#[derive(Debug)]
pub enum Direction {
    North, // -Z
    East, // // +X
    South, // +Z
    West // -X
}

impl Direction {
    // Index is just the degrees / 90
    // Eg 0 = 0 degrees, 1 = 90 deg, 2 = 180 deg, 3 = 270 deg
    pub fn rotate_by_index(&self, index: usize) -> Direction {
        match index % 4 {
            0 => {
                match self {
                    Direction::North => Direction::North,
                    Direction::East => Direction::East,
                    Direction::South => Direction::South,
                    Direction::West => Direction::West,
                }
            },
            1 => {
                match self {
                    Direction::North => Direction::East,
                    Direction::East => Direction::South,
                    Direction::South => Direction::West,
                    Direction::West => Direction::North,
                }
            }
            2 => {
                match self {
                    Direction::North => Direction::South,
                    Direction::East => Direction::West,
                    Direction::South => Direction::North,
                    Direction::West => Direction::East,
                }
            }
            3 => {
                match self {
                    Direction::North => Direction::West,
                    Direction::East => Direction::North,
                    Direction::South => Direction::East,
                    Direction::West => Direction::South,
                }
            }
            _ => unreachable!()
               
        }
    }

    pub fn from_degrees(&self, degrees: usize) -> Direction {
        match degrees {
            0 => Direction::North,
            90 => Direction::East,
            180 => Direction::South,
            270 => Direction::West,
            _ => unimplemented!()
        }
    }

    pub fn get_offset(&self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::East => (1, 0),
            Direction::South => (0, 1),
            Direction::West => (-1, 0),
        }
    }
}
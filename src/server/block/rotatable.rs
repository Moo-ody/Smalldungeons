use crate::server::utils::direction::Direction;

pub trait Rotatable {
    fn rotate(&self, direction: Direction) -> Self;
}

impl Rotatable for f32 {
    fn rotate(&self, dir: Direction) -> f32 {
        let offset = match dir {
            Direction::North => 0.0,
            Direction::East  => 90.0,
            Direction::South => 180.0,
            Direction::West  => -90.0,
            Direction::Up | Direction::Down => 0.0,
        };
        let mut result = self + offset;
        result = ((result + 180.0) % 360.0) - 180.0;
        // println!("yaw {self} offset {offset} result {result}");
        result
    }
}
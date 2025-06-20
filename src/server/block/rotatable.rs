use crate::server::utils::direction::Direction;

pub trait Rotatable {
    fn rotate(&self, direction: Direction) -> Self;
}
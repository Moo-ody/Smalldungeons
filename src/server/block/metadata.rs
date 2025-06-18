// use crate::server::utils::direction::Direction;

pub trait BlockMetadata {

    fn meta_size() -> u8;

    fn get_meta(&self) -> u8;
    
    fn from_meta(meta: u8) -> Self;

}

#[repr(transparent)]
pub struct u2(pub u8);

impl From<u8> for u2 {
    fn from(value: u8) -> Self {
        u2(value & 0b11)
    }
}

impl BlockMetadata for u2 {
    fn meta_size() -> u8 {
        2
    }
    fn get_meta(&self) -> u8 {
        self.0 & 0b11
    }
    fn from_meta(meta: u8) -> Self {
        (meta & 0b11).into()
    }
}

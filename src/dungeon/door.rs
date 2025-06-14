use crate::server::block::block_parameter::Axis;


pub enum DoorType {
    NORMAL,
    ENTRANCE,
    WITHER,
    BLOOD,
}

pub struct Door {
    pub x: i32,
    pub z: i32,

    pub direction: Axis,
    pub door_type: DoorType,
}

impl Door {
        
}
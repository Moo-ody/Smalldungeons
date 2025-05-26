use rand::Rng;

///
/// there needs to be some entity trait or something with all this default stuff
/// 
pub struct PlayerEntity {
    pub entity_id: i32,

    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

impl PlayerEntity {
    pub fn new() -> PlayerEntity {
        PlayerEntity {
            entity_id: rand::rng().random_range(1..=i32::MAX),
            pos_x: 0.0,
            pos_y: 64.0,
            pos_z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}
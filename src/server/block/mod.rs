


#[derive(PartialEq)]
pub enum Blocks {
    Air,
    Stone,
    // etc
}

// todo: need some reasonable block id + metadata method, in minecraft it is a hashmap

impl Blocks {
    pub fn block_state_id(&self) -> u16 {
        match self {
            Blocks::Air => 0,
            Blocks::Stone => (1 << 4) | 0,
        }
    }
}
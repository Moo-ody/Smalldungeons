use crate::server::chunk::chunk_section::ChunkSection;

pub mod chunk_section;

pub struct Chunk {
    pub pos_x: i32,
    pub pos_z: i32,
    pub chunk_sections: [Option<ChunkSection>; 16],
}

impl Chunk {
    
    pub fn new(pos_x: i32, pos_z: i32) -> Chunk {
        Self {
            pos_x,
            pos_z,
            chunk_sections: [
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ]
        }
    }
    
    pub fn add_section(&mut self, chunk_section: ChunkSection, index: usize) {
        self.chunk_sections[index] = Some(chunk_section);
    }
}
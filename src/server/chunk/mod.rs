use crate::server::chunk::chunk_section::ChunkSection;

pub mod chunk_section;
pub mod chunk_grid;

/// Represents a minecraft chunk.
///
/// A chunk is composed of 16 [chunk sections][ChunkSection].
/// (Note: to save on resources, if it is fully empty a chunk section is null)
pub struct Chunk {
    pub pos_x: i32,
    pub pos_z: i32,
    pub chunk_sections: [Option<ChunkSection>; 16],
}

impl Chunk {
    /// Creates an empty chunk at the X and Z coordinates provided.
    ///
    /// The chunk is entirely empty, and block data must be added with chunk sections.
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

    /// Adds a [chunk section][Chunk Section] to the provided index in the chunk.
    ///
    /// The y position of the chunk is based on the provided index,
    /// where index represents chunk sections bottom to top.
    pub fn add_section(&mut self, chunk_section: ChunkSection, index: usize) {
        self.chunk_sections[index] = Some(chunk_section);
    }

    pub fn get_section(&self, index: usize) -> &Option<ChunkSection> {
        self.chunk_sections.get(index).unwrap()
    }

    pub fn get_or_make_section(&mut self, index: usize) -> &mut Option<ChunkSection> {
        if self.chunk_sections[index].is_none() {
            self.chunk_sections[index] = Some(ChunkSection::new());
        }
        &mut self.chunk_sections[index]
    }
}
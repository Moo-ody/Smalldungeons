use crate::net::protocol::play::clientbound::ChunkData;
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
    
    pub fn get_chunk_data(&self, new: bool) -> ChunkData {
        let mut bitmask = 0u16;
        
        for section_index in 0..16 {
            if let Some(section) = &self.chunk_sections[section_index] {
                if !section.is_empty() {
                    bitmask |= 1 << section_index;
                }
            }
        }
        
        // idk if this fixes lighting issue. 
        // I think client checks if lighting is invalid, if it is it calculates it itself
        let section_count = if !new { bitmask.count_ones() as usize } else { 16 };
        let data_size: usize = section_count * 12288 + if new { 256 } else { 0 };
        
        let mut data = vec![0u8; data_size];
        let mut offset = 0;

        for section in self.chunk_sections.iter().flatten() {
            if section.is_empty() && !new {
                continue
            }
            for block in section.data {
                data[offset] = (block & 0xFF) as u8;
                data[offset + 1] = ((block >> 8) & 0xFF) as u8;
                offset += 2;
            }
        };

        // for sky and block lights,
        // it is expensive (and potentially unnecessary) to calculate lighting it will be hard-coded.
        // this might change in the future, however it is very unlikely
        if section_count != 0 {
            for _ in 0..4096 {
                data[offset] = 255;
                offset += 1;
            }
        }
        if new {
            for _ in 0..256 {
                data[offset] = 1;
                offset += 1;
            }
        }
        
        ChunkData {
            chunk_x: self.pos_x,
            chunk_z: self.pos_z,
            is_new_chunk: new,
            bitmask,
            data,
        }
    }
}
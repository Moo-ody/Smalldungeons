use crate::server::block::blocks::Blocks;
use crate::server::chunk::Chunk;

/// Chunk grid
///
/// Stores a square grid of chunks, based on the provided size.
/// It can't (and shouldn't be able to grow).
pub struct ChunkGrid {
    pub chunks: Vec<Chunk>,
    pub size: usize,
}

impl ChunkGrid {

    pub fn new(size: usize) -> ChunkGrid {
        let mut vec = Vec::with_capacity(size * size);
        for z in 0..size {
            for x in 0..size {
                vec.push(Chunk::new(x as i32, z as i32))
            }
        }
        ChunkGrid {
            chunks: vec,
            size,
        }
    }

    pub fn get_block_at(&self, x: i32, y: i32, z: i32) -> Blocks {
        if !self.is_block_valid(x, y, z) {
            return Blocks::Air;
        }

        let chunk_x = x >> 4;
        let chunk_z = z >> 4;

        if let Some(chunk) = self.get_chunk(chunk_x, chunk_z) {
            let section_index = (y / 16) as usize;
            if let Some(section) = chunk.get_section(section_index) {
                let local_x = x & 15;
                let local_y = y & 15;
                let local_z = z & 15;
                return section.get_block_at(local_x, local_y, local_z);
            }
        }
        Blocks::Air
    }

    pub fn set_block_at(&mut self, block: Blocks, x: i32, y: i32, z: i32) {
        if !self.is_block_valid(x, y, z) {
            return;
        }

        let chunk_x = x >> 4;
        let chunk_z = z >> 4;

        if let Some(chunk) = self.get_chunk_mut(chunk_x, chunk_z) {
            let section_index = (y / 16) as usize;
            if let Some(section) = chunk.get_or_make_section(section_index) {
                let local_x = x & 15;
                let local_y = y & 15;
                let local_z = z & 15;
                section.set_block_at(block.clone(), local_x, local_y, local_z);
            }
        }
    }

    pub fn set_block_at_raw() {

    }

    /// checks is block is a valid block within the chunk grid.
    fn is_block_valid(&self, x: i32, y: i32, z: i32) -> bool {
        let size = self.size as i32;
        x >= 0 && (x >> 4) < size && y >= 0 && y < 256 && z >= 0 && (z >> 4) < size
    }

    /// returns the chunk at the x and z coordinates provided
    ///
    /// using directly is not safe,
    /// however if you checked, if the coordinates are valid, it shouldn't error.
    fn get_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<&Chunk> {
        self.chunks.get(chunk_z as usize * self.size + chunk_x as usize)
    }

    fn get_chunk_mut(&mut self, chunk_x: i32, chunk_z: i32) -> Option<&mut Chunk> {
        self.chunks.get_mut(chunk_z as usize * self.size + chunk_x as usize)
    }
}
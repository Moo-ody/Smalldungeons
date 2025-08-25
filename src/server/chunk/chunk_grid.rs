use crate::net::protocol::play::clientbound::BlockChange;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::chunk::chunk::Chunk;
use crate::server::world::VIEW_DISTANCE;

/// Chunk grid
///
/// Stores a square grid of chunks, based on the provided size.
pub struct ChunkGrid {
    pub chunks: Vec<Chunk>,
    pub size: usize,

    index_offset_x: usize,
    index_offset_z: usize,
}

impl ChunkGrid {

    pub fn new(size: usize, offset_x: usize, offset_z: usize) -> ChunkGrid {
        let mut chunks = Vec::with_capacity(size * size);
        for _ in 0..size * size {
            chunks.push(Chunk::new());
        }
        ChunkGrid {
            chunks,
            size,
            index_offset_x: offset_x,
            index_offset_z: offset_z,
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
            if let Some(section) = chunk.get_or_put_section(section_index) {
                let local_x = x & 15;
                let local_y = y & 15;
                let local_z = z & 15;
                section.set_block_at(block, local_x, local_y, local_z);
            }
            chunk.packet_buffer.write_packet(&BlockChange {
                block_pos: BlockPos::new(x, y, z),
                block_state: block.get_block_state_id(),
            })
        }
    }

    /// checks is block is a valid block within the chunk grid.
    fn is_block_valid(&self, x: i32, y: i32, z: i32) -> bool {
        let size = self.size as i32;
        let chunk_x = (x >> 4) + self.index_offset_x as i32;
        let chunk_z = (z >> 4) + self.index_offset_z as i32;
        y >= 0 && y < 256 && chunk_x >= 0 && chunk_x < size && chunk_z >= 0 && chunk_z < size
    }

    /// returns the chunk at the x and z coordinates provided, none if no chunk is present
    pub fn get_chunk(&self, chunk_x: i32, chunk_z: i32) -> Option<&Chunk> {
        let size = self.size as i32;
        let x = chunk_x + self.index_offset_x as i32;
        let z = chunk_z + self.index_offset_z as i32;
        if x < 0 || z < 0 || x >= size || z >= size {
            return None
        }
        self.chunks.get(z as usize * self.size + x as usize)
    }

    fn get_chunk_mut(&mut self, chunk_x: i32, chunk_z: i32) -> Option<&mut Chunk> {
        let size = self.size as i32;
        let x = chunk_x + self.index_offset_x as i32;
        let z = chunk_z + self.index_offset_z as i32;
        if x < 0 || z < 0 || x >= size || z >= size {
            return None
        }
        self.chunks.get_mut(z as usize * self.size + x as usize)
    }
}

pub enum ChunkDiff {
    New,
    Old,
}

// maybe clamp bounds
pub fn for_each_diff<F>(
    new: (i32, i32),
    old: (i32, i32),
    mut callback: F,
) where
    F: FnMut(i32, i32, ChunkDiff),
{
    let view_distance = VIEW_DISTANCE as i32;

    let min_x = new.0 - view_distance;
    let min_z = new.1 - view_distance;
    let max_x = new.0 + view_distance;
    let max_z = new.1 + view_distance;

    let old_min_x = old.0 - view_distance;
    let old_min_z = old.1 - view_distance;
    let old_max_x = old.0 + view_distance;
    let old_max_z = old.1 + view_distance;
    
    // it would be more optimal to loop over chunks that are only different
    for x in min_x..=max_x {
        for z in min_z..=max_z {
            let in_old_range =
                x >= old_min_x && x <= old_max_x &&
                z >= old_min_z && z <= old_max_z;

            if !in_old_range {
                callback(x, z, ChunkDiff::New);
            }
        }
    }
    
    for x in old_min_x..=old_max_x {
        for z in old_min_z..=old_max_z {
            let in_new_range =
                x >= min_x && x <= max_x &&
                z >= min_z && z <= max_z;
    
            if !in_new_range {
                callback(x, z, ChunkDiff::Old);
            }
        }
    }
}

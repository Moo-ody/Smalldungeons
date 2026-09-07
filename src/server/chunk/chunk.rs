use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::ChunkData;
use crate::server::chunk::chunk_section::ChunkSection;
use crate::server::entity::entity::EntityId;

/// Represents a minecraft chunk.
///
/// A chunk is composed of 16 [chunk sections][ChunkSection].
pub struct Chunk {
    pub chunk_sections: [Option<ChunkSection>; 16],
    pub packet_buffer: PacketBuffer,
    pub entities: Vec<EntityId>
}

impl Chunk {
    
    /// Creates an empty chunk at the X and Z coordinates provided.
    ///
    /// The chunk is entirely empty, and block data must be added with chunk sections.
    pub fn new() -> Chunk {
        Self {
            chunk_sections: [
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ],
            packet_buffer: PacketBuffer::new(),
            entities: Vec::new(),
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
    
    pub fn get_or_put_section(&mut self, index: usize) -> &mut Option<ChunkSection> {
        if self.chunk_sections[index].is_none() {
            self.chunk_sections[index] = Some(ChunkSection::new());
        }
        &mut self.chunk_sections[index]
    }

    pub fn insert_entity(&mut self, entity_id: EntityId) {
        self.entities.push(entity_id);
    }
    
    pub fn remove_entity(&mut self, entity_id: &EntityId) {
        if let Some(index) = self.entities.iter().position(|id| *id == *entity_id) {
            self.entities.remove(index);
        }
    }

    
    pub fn get_chunk_data(&self, x: i32, z: i32, new: bool) -> ChunkData {
        let mut bitmask = 0u16;

        for section_index in 0..16 {
            if let Some(section) = &self.chunk_sections[section_index] {
                if !section.is_empty() {
                    bitmask |= 1 << section_index;
                }
            }
        }
        
        let section_count = bitmask.count_ones() as usize;
        let data_size: usize = section_count * 12288 + if new { 256 } else { 0 };
        
        let mut data = vec![0u8; data_size];
        let mut offset = 0;

        for section in self.chunk_sections.iter().flatten() {
            if section.is_empty() {
                continue
            }
            for block in section.data {
                data[offset] = (block & 0xFF) as u8;
                data[offset + 1] = ((block >> 8) & 0xFF) as u8;
                offset += 2;
            }
        };
        
        // All blocks have max skylight and regular light. Real 1.8 chunk data layout groups by
        // *type* across the whole packet, not interleaved per section: every active section's
        // block-type array, then every active section's block-light array, then every active
        // section's sky-light array - each light array is its own 2048-byte nibble array, so a
        // chunk with N active sections needs N*2048 bytes of block light and N*2048 of sky
        // light, not a flat 4096 total regardless of N. Writing only 4096 bytes here (this loop
        // used to run exactly once no matter how many sections were active) left every section
        // past the first with no light data at all for any chunk taller than 16 blocks - i.e.
        // basically every real room - corrupting the rest of the packet's layout (the `new`
        // chunk's biome array below landed at the wrong offset, and the true tail bytes stayed
        // zeroed). The client would flag such a chunk's lighting invalid and recompute it,
        // which is exactly the already-suspected "clients recalculate light... causing massive
        // fps drops" - and also why a forced full relight (F3+A) "fixes" rendering in it: F3+A
        // discards the corrupted data and relights from scratch instead.
        if section_count != 0 {
            for _ in 0..section_count * 2048 {
                data[offset] = 255; // block light
                offset += 1;
            }
            for _ in 0..section_count * 2048 {
                data[offset] = 255; // sky light
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
            chunk_x: x,
            chunk_z: z,
            is_new_chunk: new,
            bitmask,
            data,
        }
    }
}
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::var_int::VarInt;
use crate::server::chunk::Chunk;
use tokio::io::{AsyncWrite, AsyncWriteExt};

/// Represents Minecraft's [S21PacketChunkData](https://github.com/Marcelektro/MCP-919/blob/main/src/minecraft/net/minecraft/network/play/server/S21PacketChunkData.java).
/// 
/// Packet structure:
/// - chunk_x: i32
/// - chunk_z: i32
/// - is_new_chunk: bool
/// - bitmask: u16
/// - data length: VarInt
/// - data: array of chunk sections
/// - biome data (included in data), if chunk is new this is included (256 bytes size)
/// 
/// a chunk section is represented as:
/// block ids (with metadata): u16 array with a size of 4096
/// block lighting: u4 array with a size of 2048
/// sky lighting: u4 array with a size of 2048
/// 
#[derive(Debug, Clone)]
pub struct ChunkData {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub new_chunk: bool,
    pub bitmask: u16,
    pub data: Vec<u8>,
}

impl ChunkData {
    /// generates ChunkData packet from a [Chunk],
    /// 
    /// this function hardcodes both block and sky lighting to be the max (15).
    /// it also hardcodes the biome to be forest.
    pub fn from_chunk(chunk: &Chunk, new: bool) -> ChunkData {
        let mut bitmask = 0u16;
        
        for section_index in 0..16 {
            if let Some(section) = &chunk.chunk_sections[section_index] {
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

        for section in chunk.chunk_sections.iter().flatten() {
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
            chunk_x: chunk.pos_x,
            chunk_z: chunk.pos_z,
            new_chunk: new,
            bitmask,
            data,
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for ChunkData {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> tokio::io::Result<()> {
        let buf = build_packet!(
            0x21,
            self.chunk_x,
            self.chunk_z,
            self.new_chunk,
            self.bitmask,
            VarInt(self.data.len() as i32),
            self.data.as_slice(),
        );
        
        writer.write_all(&buf).await
    }
}

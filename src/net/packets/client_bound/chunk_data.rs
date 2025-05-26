use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacket;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::net::varint::VarInt;
use crate::server::chunk::Chunk;

#[derive(Debug)]
pub struct ChunkData {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub full_chunk: bool,
    pub data: ExtractedChunkData,
}

impl ChunkData {
    pub fn from_chunk(chunk: Chunk) -> ChunkData {
        ChunkData {
            chunk_x: chunk.pos_x,
            chunk_z: chunk.pos_z,
            full_chunk: todo!(),
            data: todo!()
        }
    }
    
    pub fn new() -> ChunkData {
        ChunkData {
            chunk_x: 0,
            chunk_z: 0,
            full_chunk: true,
            data: ExtractedChunkData {
                data: Vec::new(),
                size: 0,
            }
        }
    }
}

#[derive(Debug)]
pub struct ExtractedChunkData {
    pub data: Vec<u8>,
    pub size: i16,
}

#[async_trait::async_trait]
impl ClientBoundPacket for ChunkData {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> tokio::io::Result<()> {
        let buf = build_packet!(
            0x21,
            self.chunk_x,
            self.chunk_z,
            self.full_chunk,
            (self.data.size as i32 & 65535) as i16,
            self.data.data.as_slice(),
            0u8 // force another bit of data? i dont know whats wrong here.
        );

        let hex_string: String = buf.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(" ");

        println!("Raw bytes [{}]: {}", buf.len(), hex_string);

        writer.write_all(&buf).await
    }
}
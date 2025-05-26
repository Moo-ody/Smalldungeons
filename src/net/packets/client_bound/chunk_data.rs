use tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacket;

#[derive(Debug)]
pub struct ChunkData {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub full_chunk: bool,
    pub data: ExtractedChunkData,// Compressed chunk data
}

impl ChunkData {
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
            self.data.size,
            self.data.data.as_slice()
            //self.data
        );

        let hex_string: String = buf.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<String>>()
            .join(" ");

        println!("Raw bytes [{}]: {}", buf.len(), hex_string);
        
        writer.write_all(&buf).await
    }
}
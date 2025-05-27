// todo: do

// pub struct MapChunkBulk {
//     pub x_positions: Vec<i32>,
//     pub z_positions: Vec<i32>,
//     pub chunk_data: Vec<ExtractedChunkData>,
//     pub overworld: bool,
// }

// impl MapChunkBulk {
//     pub fn new_empty() -> MapChunkBulk {
//         MapChunkBulk {
//             x_positions: Vec::,
//             z_positions: Vec::new(),
//             chunk_data: Vec::new(),
//             overworld: true,
//         }   
//     }
// }

// #[async_trait::async_trait]
// impl ClientBoundPacket for MapChunkBulk {
//     async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
//         build_packet macro doesnt have support for these for loops i think so for now this has to be manually written
        
        // let mut buf: Vec<u8> = Vec::new();
        // let mut payload = Vec::new();

        // write_varint(&mut payload, 0x26);
        // PacketWrite::write(&self.overworld, &mut payload);
        // write_varint(&mut payload, self.chunk_data.len() as i32);
        
        // for i in 0..self.x_positions.len() {
        //     PacketWrite::write(&self.x_positions[i], &mut payload);
        //     PacketWrite::write(&self.z_positions[i], &mut payload);
        //     PacketWrite::write(&self.chunk_data[i].size, &mut payload);
        // }

        // for i in 0..self.x_positions.len() {
        //     PacketWrite::write(&self.chunk_data[i].data.as_slice(), &mut payload);
        // }
        
        // write_varint(&mut buf, payload.len() as i32);
        // buf.extend_from_slice(&payload);
        
        // writer.write_all(&buf).await
    // }
// }
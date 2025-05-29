// use crate::net::packets::packet::PacketWrite;
// use crate::server::utils::nbt_old::nbt_base::NBTBase;
// use crate::server::utils::nbt_old::nbt_size_tracker::NBTSizeTracker;
// use bytes::Buf;
// 
// #[derive(Debug, Clone)]
// pub struct NBTTagi8Array {
//     value: Vec<u8>
// }
// 
// impl NBTTagi8Array {
//     pub fn new() -> Self {
//         NBTTagi8Array {
//             value: Vec::new()
//         }
//     }
// }
// 
// impl PartialEq<Self> for NBTTagi8Array {
//     fn eq(&self, other: &Self) -> bool {
//         self.value == other.value
//     }
// }
// 
// impl NBTBase for NBTTagi8Array {
//     fn get_id(&self) -> i8 {
//         7
//     }
//     
//     fn write(&self, output: &mut Vec<u8>) {
//         PacketWrite::write(&(self.value.len() as i32), output);
//         self.value.as_slice().write(output);
//     }
// 
//     fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
//         size_tracker.read(192)?;
//         let i = input.get_i32();
//         size_tracker.read(8 * i as i64)?;
//         PacketWrite::write(input, &mut self.value);
//         Ok(())
//     }
// 
//     fn clone(&self) -> Self {
//         NBTTagi8Array {
//             value: self.value.clone()
//         }
//     }
// }
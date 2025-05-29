// use crate::net::packets::packet::PacketWrite;
// use crate::server::utils::nbt_old::nbt_base::NBTBase;
// use crate::server::utils::nbt_old::nbt_size_tracker::NBTSizeTracker;
// use crate::server::utils::nbt_old::nbt_type_enum::NBTTypeEnum;
// use anyhow::bail;
// use bytes::Buf;
// 
// #[derive(Clone)]
// #[derive(Debug)]
// pub struct NBTTagVec {
//     pub value: Vec<NBTTypeEnum>
// }
// 
// impl NBTTagVec {
//     pub fn new() -> NBTTagVec {
//         NBTTagVec {
//             value: Vec::new()
//         }
//     }
// }
// 
// impl NBTTagVec {
//     fn append(&mut self, item: NBTTypeEnum) -> anyhow::Result<()> {
//         let item_type = item.get_id();
//         if item_type == 0 {
//             bail!("Cannot append an end tag to a list")
//         }
//         if !self.value.is_empty() && item_type != self.value[0].get_id()  {
//             bail!("Cannot append a different type of tag to a list with an existing type!")
//         }
// 
//         self.value.push(item);
//         Ok(())
//     }
//     
//     fn set(&mut self, idx: usize, item: NBTTypeEnum) -> anyhow::Result<()> {
//         let item_type = item.get_id();
//         if item_type == 0 {
//             bail!("Cannot append an end tag to a list")
//         }
//         if idx < 0 || idx >= self.value.len() {
//             bail!("Index out of bounds!")
//         }
//         if item_type != self.value[0].get_id()  {
//             bail!("Cannot append a different type of tag to a list with an existing type!")
//         }
//         
//         self.value[idx] = item;
//         Ok(())
//     }
//     
//     fn remove_at(&mut self, idx: usize) {
//         self.value.remove(idx);
//     }
// }
// 
// impl NBTBase for NBTTagVec {
//     fn get_id(&self) -> i8 {
//         9
//     }
//     
//     fn write(&self, output: &mut Vec<u8>) {
//         let mut tag_type = 0i8;
//         if self.value.len() != 0 {
//             tag_type = self.value[0].get_id()
//         }
//         PacketWrite::write(&tag_type, output);
//         PacketWrite::write(&(self.value.len() as i32), output);
//         for tag in self.value.iter() {
//             tag.write(output);
//         }
//     }
// 
//     fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
//         size_tracker.read(296)?;
//         if depth > 512 {
//             bail!("NBT tag vec is too complex, depth > 512")
//         }
//         let tag_type = input.get_i8();
//         let len = input.get_i32();
//         if !self.value.is_empty() && tag_type > 0 && len > 0 {
//             bail!("Missing tag type!")
//         }
//         
//         size_tracker.read(32 * len as i64)?;
//         for index in 0..len {
//             let mut nbtbase = NBTTypeEnum::get_by_id(tag_type)?;
//             nbtbase.read(input, depth + 1, size_tracker)?;
//             self.value.push(nbtbase);
//         }
//         
//         Ok(())
//     }
// 
//     fn clone(&self) -> Self {
//         NBTTagVec {
//             value: self.value.clone()
//         }
//     }
// }
// use crate::server::utils::nbt_old::nbt_tag_compound::NBTTagCompound;
// use crate::server::utils::nbt_old::nbt_tag_double::NBTTagf64;
// use crate::server::utils::nbt_old::nbt_tag_end::NBTTagEnd;
// use crate::server::utils::nbt_old::nbt_tag_f32::NBTTagf32;
// use crate::server::utils::nbt_old::nbt_tag_i16::NBTTagi16;
// use crate::server::utils::nbt_old::nbt_tag_i32::NBTTagi32;
// use crate::server::utils::nbt_old::nbt_tag_i32_array::NBTTagi32Array;
// use crate::server::utils::nbt_old::nbt_tag_i64::NBTTagi64;
// use crate::server::utils::nbt_old::nbt_tag_i8::NBTTagi8;
// use crate::server::utils::nbt_old::nbt_tag_i8_array::NBTTagi8Array;
// use crate::server::utils::nbt_old::nbt_tag_list::NBTTagVec;
// use crate::server::utils::nbt_old::nbt_tag_string::NBTTagString;
// use anyhow::bail;
// use enum_dispatch::enum_dispatch;
// 
// #[enum_dispatch(NBTBase)]
// #[derive(Clone, Debug)]
// pub enum NBTTypeEnum {
//     NBTTagi8,
//     NBTTagi8Array,
//     NBTTagCompound,
//     NBTTagf64,
//     NBTTagf32,
//     NBTTagi32,
//     NBTTagi32Array,
//     NBTTagVec,
//     NBTTagEnd,
//     NBTTagi64,
//     NBTTagi16,
//     NBTTagString,
// }
// 
// impl NBTTypeEnum {
//     pub fn get_by_id(id: i8) -> anyhow::Result<NBTTypeEnum> {
//         Ok(match id {
//             0 => NBTTypeEnum::from(NBTTagEnd),
//             1 => NBTTypeEnum::from(NBTTagi8::new()),
//             2 => NBTTypeEnum::from(NBTTagi16::new()),
//             3 => NBTTypeEnum::from(NBTTagi32::new()),
//             4 => NBTTypeEnum::from(NBTTagi64::new()),
//             5 => NBTTypeEnum::from(NBTTagf32::new()),
//             6 => NBTTypeEnum::from(NBTTagf64::new()),
//             7 => NBTTypeEnum::from(NBTTagi8Array::new()),
//             8 => NBTTypeEnum::from(NBTTagString::new()),
//             9 => NBTTypeEnum::from(NBTTagVec::new()),
//             10 => NBTTypeEnum::from(NBTTagCompound::new()),
//             11 => NBTTypeEnum::from(NBTTagi32Array::new()),
//             _ => bail!("Unknown NBT type id: {}", id),
//         })
//     }
// }
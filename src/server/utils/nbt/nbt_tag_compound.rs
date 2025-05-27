use std::collections::HashMap;
use anyhow::bail;
use bytes::Buf;
use crate::net::packets::packet::PacketWrite;
use crate::server::utils::nbt::nbt_base::NBTBase;
use crate::server::utils::nbt::nbt_size_tracker::NBTSizeTracker;
use crate::server::utils::nbt::nbt_type_enum::NBTTypeEnum;

#[derive(Clone)]
pub struct NBTTagCompound {
    tag_map: HashMap<String, NBTTypeEnum>
}

impl NBTTagCompound {
    pub(crate) fn new() -> NBTTagCompound {
        NBTTagCompound {
            tag_map: HashMap::new()
        }
    }
    
    fn put(&mut self, key: String, value: NBTTypeEnum) {
        self.tag_map.insert(key, value);
    }
}

fn readNBT(id: i8, key: &String, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<NBTTypeEnum> {
    let mut base = NBTTypeEnum::get_by_id(id)?;
    base.read(input, depth, size_tracker)?;
    Ok(base)
}

impl NBTBase for NBTTagCompound {
    fn get_id(&self) -> i8 {
        10
    }
    
    fn write(&self, output: &mut Vec<u8>) {
        for (key, value) in self.tag_map.iter() {
            let id = &value.get_id();
            PacketWrite::write(id, output);
            if id *id != 0i8 {
                PacketWrite::write(key, output);
                value.write(output);
            }
        }
        output.push(0);
        todo!()
    }

    fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
        size_tracker.read(384)?;
        if depth > 512 {
            bail!("NBT tag is too complex, depth > 512");
        }
        self.tag_map.clear();
        
        let mut iters = 0;
        loop {
            if iters > 512 {
                bail!("Too many iterations! somethings wrong!")
            }
            
            if input.get_u8() == 0 {
                break;
            }
            let key = String::from_utf8(input.get_u16_le().to_le_bytes().to_vec())?;
            size_tracker.read((224 + 16 * key.len()) as i64)?;
            let base = readNBT(input.get_i8(), &key, input, depth + 1, size_tracker)?;
            self.put(key, base);
            size_tracker.read(288)?;
            
            iters += 1;
        }
        Ok(())
    }

    fn clone(&self) -> Self {
        todo!()
    }
}
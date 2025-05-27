use bytes::Buf;
use crate::net::packets::packet::PacketWrite;
use crate::server::utils::nbt::nbt_base::NBTBase;
use crate::server::utils::nbt::nbt_size_tracker::NBTSizeTracker;

#[derive(Debug, Clone)]
pub struct NBTTagi32Array {
    value: Vec<i32>
}

impl NBTTagi32Array {
    pub fn new() -> Self {
        NBTTagi32Array {
            value: Vec::new()
        }
    }
}

impl PartialEq<Self> for NBTTagi32Array {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl NBTBase for NBTTagi32Array {
    fn get_id(&self) -> i8 {
        11
    }
    
    fn write(&self, output: &mut Vec<u8>) {
        PacketWrite::write(&(self.value.len() as i32), output);
        for i in self.value.iter() {    
            i.write(output); 
        }
    }

    fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
        size_tracker.read(192)?;
        let i = input.get_i32();
        size_tracker.read(32 * i as i64)?;
        for j in 0..i {
            self.value.push(input.get_i32());
        }
        Ok(())
    }

    fn clone(&self) -> Self {
        NBTTagi32Array {
            value: self.value.clone()
        }
    }
}
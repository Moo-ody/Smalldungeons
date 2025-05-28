use crate::server::utils::nbt_old::nbt_base::NBTBase;
use crate::server::utils::nbt_old::nbt_size_tracker::NBTSizeTracker;
use bytes::Buf;

#[derive(Debug, Clone)]
pub struct NBTTagi8 {
    value: i8
}

impl NBTTagi8 {
    pub fn new() -> Self {
        NBTTagi8 {
            value: 0
        }
    }
}

impl PartialEq<Self> for NBTTagi8 {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl NBTBase for NBTTagi8 {
    fn get_id(&self) -> i8 {
        1
    }
    
    fn write(&self, output: &mut Vec<u8>) {
        output.push(self.value as u8)
    }

    fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
        size_tracker.read(72)?;
        self.value = input.get_i8();
        Ok(())
    }

    fn clone(&self) -> Self {
        NBTTagi8 {
            value: self.value
        }
    }
}
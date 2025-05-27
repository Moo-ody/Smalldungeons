use bytes::Buf;
use crate::net::packets::packet::PacketWrite;
use crate::server::utils::nbt::nbt_base::NBTBase;
use crate::server::utils::nbt::nbt_size_tracker::NBTSizeTracker;

#[derive(Debug, Clone)]
pub struct NBTTagi32 {
    value: i32
}

impl NBTTagi32 {
    pub fn new() -> Self {
        NBTTagi32 {
            value: 0
        }
    }
}

impl PartialEq<Self> for NBTTagi32 {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl NBTBase for NBTTagi32 {
    fn get_id(&self) -> i8 {
        3
    }
    
    fn write(&self, output: &mut Vec<u8>) {
        self.value.write(output)
    }

    fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
        size_tracker.read(96)?;
        self.value = input.get_i32();
        Ok(())
    }

    fn clone(&self) -> Self {
        NBTTagi32 {
            value: self.value
        }
    }
}
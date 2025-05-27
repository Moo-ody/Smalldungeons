use bytes::Buf;
use crate::net::packets::packet::PacketWrite;
use crate::server::utils::nbt::nbt_base::NBTBase;
use crate::server::utils::nbt::nbt_size_tracker::NBTSizeTracker;

#[derive(Debug, Clone)]
pub struct NBTTagf32 {
    value: f32
}

impl NBTTagf32 {
    pub fn new() -> Self {
        NBTTagf32 {
            value: 0.0
        }
    }
}

impl PartialEq<Self> for NBTTagf32 {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl NBTBase for NBTTagf32 {
    fn get_id(&self) -> i8 {
        5
    }
    
    fn write(&self, output: &mut Vec<u8>) {
        PacketWrite::write(&self.value, output)
    }

    fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
        size_tracker.read(96)?;
        self.value = input.get_f32();
        Ok(())
    }

    fn clone(&self) -> Self {
        NBTTagf32 {
            value: self.value
        }
    }
}
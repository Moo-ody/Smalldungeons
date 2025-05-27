use bytes::Buf;
use crate::net::packets::packet::PacketWrite;
use crate::server::utils::nbt::nbt_base::NBTBase;
use crate::server::utils::nbt::nbt_size_tracker::NBTSizeTracker;

#[derive(Debug, Clone)]
pub struct NBTTagEnd;

impl NBTTagEnd {
}

impl NBTBase for NBTTagEnd {
    fn get_id(&self) -> i8 {
        0
    }
    
    fn write(&self, output: &mut Vec<u8>) {}

    fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
        size_tracker.read(64)?;
        Ok(())
    }

    fn clone(&self) -> Self {
        NBTTagEnd
    }
}
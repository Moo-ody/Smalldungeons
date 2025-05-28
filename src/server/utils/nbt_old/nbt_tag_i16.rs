use crate::net::packets::packet::PacketWrite;
use crate::server::utils::nbt_old::nbt_base::NBTBase;
use crate::server::utils::nbt_old::nbt_size_tracker::NBTSizeTracker;
use bytes::Buf;

#[derive(Clone)]
#[derive(Debug)]
pub struct NBTTagi16 {
    value: i16
}

impl NBTTagi16 {
    pub fn new() -> NBTTagi16 {
        NBTTagi16 { 
            value: 0 
        }
    }
}

impl NBTBase for NBTTagi16 {
    fn get_id(&self) -> i8 {
        2
    }

    fn write(&self, output: &mut Vec<u8>) {
        self.value.write(output);
    }

    fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
        size_tracker.read(80)?;
        self.value = input.get_i16();
        Ok(())
    }

    fn clone(&self) -> Self {
        todo!()
    }
}
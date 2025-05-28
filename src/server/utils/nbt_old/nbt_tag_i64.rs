use crate::net::packets::packet::PacketWrite;
use crate::server::utils::nbt_old::nbt_base::NBTBase;
use crate::server::utils::nbt_old::nbt_size_tracker::NBTSizeTracker;
use bytes::Buf;

#[derive(Clone)]
#[derive(Debug)]
pub struct NBTTagi64 {
    value: i64
}

impl NBTTagi64 {
    pub fn new() -> NBTTagi64 {
        NBTTagi64 {
            value: 0
        }
    }
}

impl NBTBase for NBTTagi64 {
    fn get_id(&self) -> i8 {
        4
    }

    fn write(&self, output: &mut Vec<u8>) {
        self.value.write(output)
    }

    fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
        size_tracker.read(128)?;
        self.value = input.get_i64();
        Ok(())
    }

    fn clone(&self) -> Self {
        todo!()
    }
}
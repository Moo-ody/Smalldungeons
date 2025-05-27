use bytes::Buf;
use crate::net::packets::packet::PacketWrite;
use crate::server::utils::nbt::nbt_base::NBTBase;
use crate::server::utils::nbt::nbt_size_tracker::NBTSizeTracker;

#[derive(Clone)]
pub struct NBTTagString {
    value: String,
}

impl NBTTagString {
    pub fn new() -> NBTTagString {
        NBTTagString { 
            value: String::new()
        }
    }
}

impl NBTBase for NBTTagString {
    fn get_id(&self) -> i8 {
        8
    }

    fn write(&self, output: &mut Vec<u8>) {
        self.value.as_bytes().write(output);
    }

    fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()> {
        size_tracker.read(288)?;
        self.value = String::from_utf8(input.get_u16_le().to_le_bytes().to_vec())?;
        size_tracker.read(16 * self.value.len() as i64)?;
        Ok(())
    }

    fn clone(&self) -> Self {
        todo!()
    }
}
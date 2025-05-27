use enum_dispatch::enum_dispatch;
use crate::server::utils::nbt::nbt_size_tracker::NBTSizeTracker;

#[enum_dispatch]
pub trait NBTBase {
    fn get_id(&self) -> i8;
    
    fn write(&self, output: &mut Vec<u8>);
    fn read(&mut self, input: &mut &[u8], depth: i32, size_tracker: &mut NBTSizeTracker) -> anyhow::Result<()>;
    
    fn clone(&self) -> Self;
}
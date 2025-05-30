use crate::net::packets::packet_write::PacketWrite;
use crate::server::utils::nbt::encode::serialize_nbt;
use crate::server::utils::nbt::NBT;

#[derive(Debug, Clone)]
pub struct ItemStack {
    pub item: i16,
    pub stack_size: i8,
    pub metadata: i16,
    pub tag_compound: Option<NBT>,
}

impl PacketWrite for ItemStack {
    fn write(&self, buf: &mut Vec<u8>) {
        self.item.write(buf);
        self.stack_size.write(buf);
        self.metadata.write(buf);

        match &self.tag_compound {
            None => { 0u8.write(buf) }
            Some(nbt) => {
                buf.extend(serialize_nbt(&nbt));
            }
        }
    }
}
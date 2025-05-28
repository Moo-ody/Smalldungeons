use crate::net::packets::packet::PacketWrite;
use crate::server::utils::nbt::{serialize, NBTNode};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

#[derive(Debug)]
pub struct ItemStack {
    pub item: i16,
    pub stack_size: u8,
    pub metadata: u16,
    pub tag_compound: Option<NBTNode>,
}

impl PacketWrite for ItemStack {
    fn write(&self, buf: &mut Vec<u8>) {
        PacketWrite::write(&self.item, buf);
        PacketWrite::write(&self.stack_size, buf);
        PacketWrite::write(&self.metadata, buf);

        if self.tag_compound.is_none() {
            PacketWrite::write(&0u8, buf);
        } else {
            if let Some(tag) = &self.tag_compound {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&serialize(tag)).unwrap();
                let compressed_data = encoder.finish().unwrap();
                buf.extend(&compressed_data);
            }
        }
    }
}
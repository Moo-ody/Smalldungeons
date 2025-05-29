use crate::net::packets::packet::PacketWrite;
use crate::server::utils::nbt::{serialize, NBTNode};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

#[derive(Debug)]
pub struct ItemStack {
    pub item: i16,
    pub stack_size: i8,
    pub metadata: i16,
    pub tag_compound: Option<NBTNode>,
}

impl PacketWrite for ItemStack {
    fn write(&self, buf: &mut Vec<u8>) {
        self.item.write(buf);
        self.stack_size.write(buf);
        self.metadata.write(buf);

        match &self.tag_compound {
            None => { 0u8.write(buf) }
            Some(tag) => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&serialize(tag)).unwrap();
                let compressed_data = encoder.finish().unwrap();
                buf.extend(&compressed_data);
            }
        }
    }
}
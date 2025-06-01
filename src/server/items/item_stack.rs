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

impl PacketWrite for Option<ItemStack> {
    fn write(&self, buf: &mut Vec<u8>) {
        if let Some(item_stack) = self {
            item_stack.item.write(buf);
            item_stack.stack_size.write(buf);
            item_stack.metadata.write(buf);

            match &item_stack.tag_compound {
                None => 0u8.write(buf),
                Some(nbt) => buf.extend(serialize_nbt(nbt)),
            }
        } else {
            (-1i16).write(buf)
        }
    }
}
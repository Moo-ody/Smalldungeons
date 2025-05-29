use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::varint::read_varint;
use crate::server::old_world::World;
use anyhow::bail;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct ClientSettings {
    pub lang: String,
    pub view_distance: u8,
    pub chat_mode: i8,
    pub chat_colors: bool,
    pub skin_parts: u8,
}

#[async_trait::async_trait]
impl ServerBoundPacket for ClientSettings {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(ClientSettings {
            lang: read_string_from_buf(buf, 7)?,
            view_distance: buf.get_u8(),
            chat_mode: buf.get_i8(),
            chat_colors: buf.get_u8() != 0,
            skin_parts: buf.get_u8(),
        })
    }
    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        Ok(())
    }
}

pub fn read_string_from_buf(buf: &mut BytesMut, max_length: i32) -> anyhow::Result<String> {
    let len = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read string length"))?;
    if len > max_length * 4 {
        bail!("String too long. {:?} / {}", len, max_length * 4);
    }
    if len < 0 {
        bail!("String length is less than 0???")
    }
    
    let string = String::from_utf8(buf.split_to(len as usize).to_vec())?;
    
    if string.len() > max_length as usize {
        bail!("String too long. {:?} > {}", len, max_length);   
    }
    Ok(string)
}
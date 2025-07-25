use crate::net::packets::old_packet::ServerBoundPacket;
use crate::net::packets::read_string_from_buf;
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
        Ok(Self {
            lang: read_string_from_buf(buf, 7)?,
            view_distance: buf.get_u8(),
            chat_mode: buf.get_i8(),
            chat_colors: buf.get_u8() != 0,
            skin_parts: buf.get_u8(),
        })
    }
}
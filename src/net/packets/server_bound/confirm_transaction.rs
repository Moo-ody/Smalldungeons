use crate::net::packets::old_packet::ServerBoundPacket;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct ConfirmTransaction {
    pub window_id: i8,
    pub action_number: i16,
    pub accepted: bool,
}

#[async_trait::async_trait]
impl ServerBoundPacket for ConfirmTransaction {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(ConfirmTransaction {
            window_id: buf.get_i8(),
            action_number: buf.get_i16(),
            accepted: buf.get_i8() != 0,
        })
    }
}
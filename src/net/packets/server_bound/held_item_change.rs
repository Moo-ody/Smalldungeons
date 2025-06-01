use crate::net::packets::packet::ServerBoundPacket;
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct HeldItemChange {
    // for some reason this is a short
    pub slot_id: u16
}

#[async_trait::async_trait]
impl ServerBoundPacket for HeldItemChange {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(HeldItemChange {
            slot_id: buf.get_u16(),
        })
    }
}
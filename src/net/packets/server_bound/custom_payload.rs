use crate::net::packets::old_packet::ServerBoundPacket;
use bytes::BytesMut;

pub struct CustomPayload;

#[async_trait::async_trait]
impl ServerBoundPacket for CustomPayload {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        todo!()
    }
}
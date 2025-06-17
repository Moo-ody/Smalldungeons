use crate::net::packets::packet::ServerBoundPacket;
use bytes::BytesMut;

#[derive(Debug)]
pub struct SwingAnimation;

#[async_trait::async_trait]
impl ServerBoundPacket for SwingAnimation {
    async fn read_from(_: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(SwingAnimation)
    }
}
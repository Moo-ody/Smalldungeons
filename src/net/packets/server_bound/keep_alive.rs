use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use bytes::BytesMut;
use crate::net::varint::read_varint;
use crate::server::world::World;

#[derive(Debug)]
pub struct KeepAlive {
    pub id: i32,
}

#[async_trait::async_trait]
impl ServerBoundPacket for KeepAlive {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        let id = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read keep alive id"))?;
        Ok(KeepAlive {
            id
        })
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        Ok(())
    }
}
use std::any::Any;
use bytes::{Buf, BytesMut};
use anyhow::{Result, bail};
use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::packet_registry::ClientBoundPackets;
use crate::net::packets::client_bound::server_info::ServerInfo;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::STATUS_RESPONSE_JSON;

#[derive(Debug)]
pub struct StatusRequest<> {}

#[async_trait::async_trait]
impl ServerBoundPacket for StatusRequest {
    async fn read_from(buf: &mut BytesMut) -> Result<Self> {
        // let packet_id = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read packet id"))?;
        // if packet_id != 0x00 {
        //     bail!("Expected handshake, received {}", packet_id)
        // }

        Ok(StatusRequest {})
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn process(&self, context: PacketContext) -> Result<()> {
        context.network_tx.send(NetworkMessage::SendPacket {
            client_id: context.client_id,
            packet: ClientBoundPackets::ServerInfo(ServerInfo {
                status: STATUS_RESPONSE_JSON.parse()?,
            }),
        })?;
        Ok(())
    }
}
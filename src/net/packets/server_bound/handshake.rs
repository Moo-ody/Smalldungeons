use std::any::Any;
use tokio::io::{AsyncRead, WriteHalf};
use bytes::{Buf, BytesMut};
use anyhow::{Result, bail};
use tokio::net::TcpStream;
use crate::net::connection_state::get_state_from_id;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::packet_registry::ClientBoundPackets;
use crate::net::packets::client_bound::server_info::ServerInfo;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::varint::read_varint;
use crate::STATUS_RESPONSE_JSON;

#[derive(Debug)]
pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: i16,
    pub next_state: i32
}

#[async_trait::async_trait]
impl ServerBoundPacket for Handshake {
    async fn read_from(buf: &mut BytesMut) -> Result<Self> {
        // let packet_id = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read packet id"))?;
        // if packet_id != 0x00 {
        //     bail!("Expected handshake, received {}", packet_id)
        // }

        let protocol_version = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read protocol version"))?;
        let addr_len = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read addr length"))?  as usize;

        if buf.len() < addr_len + 3 {
            bail!("Buffer too small for server address + port + next_state");
        }

        let server_address_bytes = buf.split_to(addr_len);
        let server_address = String::from_utf8(server_address_bytes.to_vec())
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in server address: {}", e))?;

        let server_port = buf.get_i16();

        // Next state (VarInt)
        let next_state = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read next state"))?;

        Ok(Handshake {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn process(&self, context: PacketContext) -> Result<()> {
        println!("Received handshake packet");
        
        let new_state = get_state_from_id(self.next_state);
        
        context.network_tx.send(NetworkMessage::UpdateConnectionState {
            client_id: context.client_id,
            new_state: new_state.clone(),
        })?;
        
        // if self.next_state == 1 {
        //     context.network_tx.send(NetworkMessage::SendPacket {
        //         client_id: context.client_id,
        //         packet: ClientBoundPackets::ServerInfo(ServerInfo {
        //             status: STATUS_RESPONSE_JSON.parse()?,
        //         }),
        //     })?;
        // }

        Ok(())
    }

}
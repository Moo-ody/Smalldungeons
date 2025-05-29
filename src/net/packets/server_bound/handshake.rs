use crate::net::connection_state::ConnectionState;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::varint::read_varint;
use crate::server::old_world::World;
use anyhow::{bail, Result};
use bytes::{Buf, BytesMut};

#[derive(Debug)]
pub struct Handshake {
    pub _protocol_version: i32,
    pub _server_address: String,
    pub _server_port: i16,
    pub next_state: i32
}

#[async_trait::async_trait]
impl ServerBoundPacket for Handshake {
    async fn read_from(buf: &mut BytesMut) -> Result<Self> {
        let protocol_version = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read protocol version"))?;
        let addr_len = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read addr length"))?  as usize;

        if buf.len() < addr_len + 3 {
            bail!("Buffer too small for server address + port + next_state");
        }

        let server_address_bytes = buf.split_to(addr_len);
        let server_address = String::from_utf8(server_address_bytes.to_vec())
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in server address: {}", e))?;

        let server_port = buf.get_i16();

        let next_state = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read next state"))?;

        Ok(Handshake {
            _protocol_version: protocol_version,
            _server_address: server_address,
            _server_port: server_port,
            next_state,
        })
    }
    async fn process(&self, context: PacketContext) -> Result<()> {
        println!("Received handshake packet");

        let new_state = ConnectionState::from_id(self.next_state)?;

        context.network_tx.send(NetworkMessage::UpdateConnectionState {
            client_id: context.client_id,
            new_state,
        })?;

        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> Result<()> {
        Ok(())
    }
}
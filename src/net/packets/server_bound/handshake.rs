use crate::net::connection_state::ConnectionState;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::packets::read_string_from_buf;
use crate::net::var_int::read_var_int;
use anyhow::{Context, Result};
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
        Ok(Handshake {
            _protocol_version: read_var_int(buf).context("Failed to read protocol version")?,
            _server_address: read_string_from_buf(buf, 255)?,
            _server_port: buf.get_i16(),
            next_state: read_var_int(buf).context("Failed to read next state")?,
        })
    }
    async fn process<'a>(&self, context: PacketContext<'a>) -> Result<()> {
        context.client.connection_state = ConnectionState::from_id(self.next_state)?;
        Ok(())
    }
}
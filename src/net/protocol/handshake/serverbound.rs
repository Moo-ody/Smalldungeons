use crate::net::client::Client;
use crate::net::connection_state::ConnectionState;
use crate::net::packets::packet::{ProcessContext, ProcessPacket};
use crate::net::packets::packet_deserialize::PacketDeserializable;
use crate::net::var_int::read_var_int;
use crate::register_serverbound_packets;
use anyhow::{bail, Context};
use bytes::{Buf, BytesMut};

register_serverbound_packets! {
    HandshakePacket;
    Handshake = 0x00;
}

pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: i16,
    pub next_state: i32,
}

impl PacketDeserializable for Handshake {
    fn read(buffer: &mut BytesMut) -> anyhow::Result<Self> {
        let protocol_version = read_var_int(buffer).context("Failed to read protocl version")?;
        let addr_len = read_var_int(buffer).context("Failed to read addr len")? as usize;
        
        if buffer.len() < addr_len + 3 {
            bail!("Buffer too small for server address + port + next_state");
        }
        let server_address_bytes = buffer.split_to(addr_len);
        let server_address = String::from_utf8(server_address_bytes.to_vec())?;

        let server_port = buffer.get_i16();

        let next_state = read_var_int(buffer).context("Failed to read next state")?;

        Ok(Handshake {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}

impl ProcessPacket for Handshake {
    async fn process<'a>(&self, client: &mut Client, context: ProcessContext<'a>) -> anyhow::Result<()> {
        client.connection_state = ConnectionState::from_id(self.next_state)?;
        Ok(())
    }
}
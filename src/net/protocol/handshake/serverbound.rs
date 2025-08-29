use crate::net::client::Client;
use crate::net::connection_state::ConnectionState;
use crate::net::packets::packet::{ProcessContext, ProcessPacket};
use crate::net::var_int::VarInt;
use crate::register_serverbound_packets;
use crate::server::utils::sized_string::SizedString;
use blocks::packet_deserializable;

register_serverbound_packets! {
    HandshakePacket;
    Handshake = 0x00;
}

packet_deserializable! {
    pub struct Handshake {
        pub protocol_version: VarInt,
        pub server_address: SizedString<255>,
        pub server_port: u16,
        pub next_state: VarInt,
    }
}

impl ProcessPacket for Handshake {
    async fn process<'a>(&self, client: &mut Client, context: ProcessContext<'a>) -> anyhow::Result<()> {
        client.connection_state = ConnectionState::from_id(self.next_state.0)?;
        Ok(())
    }
}
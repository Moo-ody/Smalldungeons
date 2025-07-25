use crate::net::connection_state::ConnectionState;
use crate::net::internal_packets::{MainThreadMessage, NetworkThreadMessage};
use crate::net::packets::old_packet::ServerBoundPacket;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::packets::packet_context::PacketContext;
use crate::net::packets::protocol::clientbound::LoginSuccess;
use crate::net::var_int::read_var_int;
use anyhow::Context;
use bytes::BytesMut;

#[derive(Debug)]
pub struct LoginStart {
    pub username: String,
}

#[async_trait::async_trait]
impl ServerBoundPacket for LoginStart {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        let name_len = read_var_int(buf).context("Failed to read name length")? as usize;
        let name_bytes = buf.split_to(name_len);

        let username = String::from_utf8(name_bytes.to_vec())?;
        Ok(Self { username })
    }

    async fn process<'a>(&self, context: PacketContext<'a>) -> anyhow::Result<()> {
        println!("Player {} is attempting to log in.", self.username);
        
        context.network_tx.send(NetworkThreadMessage::SendPackets {
            client_id: context.client.client_id(),
            buffer: {
                let mut buffer = PacketBuffer { buf: Vec::new() };
                // dummy uuid because we dont need auth for local
                // but should allow skin
                buffer.write_packet(&LoginSuccess {
                    uuid: "d74cb748-b23b-4a99-b41e-b85f73d41999".to_string(),
                    name: self.username.clone(),
                });
                buffer.buf
            },
        })?;

        context.client.connection_state = ConnectionState::Play;

        context.main_tx.send(MainThreadMessage::NewPlayer {
            client_id: context.client.client_id(),
            username: self.username.clone(),
        })?;

        Ok(())
    }
}
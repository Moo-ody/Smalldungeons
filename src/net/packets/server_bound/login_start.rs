use crate::net::connection_state::ConnectionState;
use crate::net::internal_packets::MainThreadMessage;
use crate::net::packets::client_bound::login_success::LoginSuccess;
use crate::net::packets::packet::{SendPacket, ServerBoundPacket};
use crate::net::packets::packet_context::PacketContext;
use crate::net::packets::read_string_from_buf;
use bytes::BytesMut;

#[derive(Debug)]
pub struct LoginStart {
    pub username: String,
}

#[async_trait::async_trait]
impl ServerBoundPacket for LoginStart {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(Self { username: read_string_from_buf(buf, 16)? })
    }

    async fn process<'a>(&self, context: PacketContext<'a>) -> anyhow::Result<()> {
        println!("Player {} is attempting to log in.", self.username);

        LoginSuccess {
            uuid: "d74cb748-b23b-4a99-b41e-b85f73d41999".to_string(), // dummy uuid because we dont need auth for local
            name: self.username.clone(),
        }.send_packet(context.client.client_id(), context.network_tx)?;

        context.client.connection_state = ConnectionState::Play;

        context.main_tx.send(MainThreadMessage::NewPlayer {
            client_id: context.client.client_id(),
            username: self.username.clone(),
        })?;

        Ok(())
    }
}
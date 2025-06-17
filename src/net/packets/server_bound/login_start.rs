use crate::net::client_event::ClientEvent;
use crate::net::connection_state::ConnectionState;
use crate::net::packets::client_bound::login_success::LoginSuccess;
use crate::net::packets::packet::{SendPacket, ServerBoundPacket};
use crate::net::packets::packet_context::PacketContext;
use crate::net::var_int::read_var_int;
use bytes::BytesMut;

#[derive(Debug)]
pub struct LoginStart {
    pub username: String,
}

#[async_trait::async_trait]
impl ServerBoundPacket for LoginStart {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        let name_len = read_var_int(buf).ok_or_else(|| anyhow::anyhow!("Failed to read name length"))?  as usize;
        let name_bytes = buf.split_to(name_len);

        let username = String::from_utf8(name_bytes.to_vec())
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in name: {}", e))?;
        Ok(Self { username })
    }

    async fn process<'a>(&self, context: PacketContext<'a>) -> anyhow::Result<()> {
        println!("Player {} is attempting to log in.", self.username);

        LoginSuccess {
            uuid: "d74cb748-b23b-4a99-b41e-b85f73d41999".to_string(), // dummy uuid because we dont need auth for local
            name: self.username.clone(),
        }.send_packet(context.client.client_id(), &context.network_tx)?;

        context.client.connection_state = ConnectionState::Play;

        context.event_tx.send(ClientEvent::NewPlayer {
            client_id: context.client.client_id(),
        })?;

        Ok(())
    }
}
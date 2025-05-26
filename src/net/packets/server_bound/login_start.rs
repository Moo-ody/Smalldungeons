use crate::net::client_event::ClientEvent;
use crate::net::connection_state::ConnectionState;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::login_success;
use crate::net::packets::client_bound::packet_registry::ClientBoundPackets::LoginSuccess;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::varint::read_varint;
use bytes::BytesMut;

#[derive(Debug)]
pub struct LoginStart {
    pub username: String,
}

#[async_trait::async_trait]
impl ServerBoundPacket for LoginStart {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        let name_len = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read name length"))?  as usize;
        let name_bytes = buf.split_to(name_len);

        let username = String::from_utf8(name_bytes.to_vec())
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in name: {}", e))?;
        Ok(LoginStart { username })
    }
    
    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        println!("Player {} is attempting to log in.", self.username);
        
        LoginSuccess(login_success::LoginSuccess {
            uuid: "d74cb748-b23b-4a99-b41e-b85f73d41999".to_string(), // dummy uuid because we dont need auth for local
            name: self.username.clone(),
        }).send_packet(context.client_id, &context.network_tx)?;
        
        context.network_tx.send(NetworkMessage::UpdateConnectionState {
            client_id: context.client_id,
            new_state: ConnectionState::Play,
        })?;
        
        context.event_tx.send(ClientEvent::NewPlayer {
            client_id: context.client_id,
        })?;

        Ok(())
    }
}
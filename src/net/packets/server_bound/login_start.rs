use std::any::Any;
use bytes::BytesMut;
use crate::net::client_event::ClientEvent;
use crate::net::connection_state::ConnectionState;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::login_success::LoginSuccess;
use crate::net::packets::client_bound::packet_registry::ClientBoundPackets;
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use crate::net::varint::read_varint;

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

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        println!("Player {} is attempting to log in.", self.username);
        
        context.network_tx.send(NetworkMessage::SendPacket {
            client_id: context.client_id,
            packet: ClientBoundPackets::LoginSuccess( LoginSuccess {
                uuid: "d74cb748-b23b-4a99-b41e-b85f73d41999".to_string(), // dummy uuid because we dont need auth for local
                name: self.username.clone(),
            })
        })?;

        // Transition connection state to Play
        context.network_tx.send(NetworkMessage::UpdateConnectionState {
            client_id: context.client_id,
            new_state: ConnectionState::Play,
        })?;
        
        context.event_tx.send(ClientEvent::NewClient {
            client_id: context.client_id,
        })?;

        Ok(())
    }
}
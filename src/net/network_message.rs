use crate::net::connection_state::ConnectionState;
use crate::net::packets::packet_registry::ClientBoundPacket;
use crate::server::player::ClientId;
use tokio::sync::oneshot::Sender;

pub enum NetworkMessage {
    SendPacket {
        client_id: ClientId,
        packet: ClientBoundPacket
    },
    DisconnectClient {
        client_id: ClientId,
    },
    UpdateConnectionState {
        client_id: ClientId,
        new_state: ConnectionState
    },
    GetConnectionState {
        client_id: ClientId,
        response: Sender<ConnectionState>
    },
}
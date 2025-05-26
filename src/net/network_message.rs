use crate::net::connection_state::ConnectionState;
use crate::net::packets::packet_registry::ClientBoundPackets;
use tokio::sync::oneshot::Sender;

pub enum NetworkMessage {
    SendPacket {
        client_id: u32,
        packet: ClientBoundPackets
    },
    DisconnectClient {
        client_id: u32,
    },
    UpdateConnectionState {
        client_id: u32,
        new_state: ConnectionState
    },
    GetConnectionState {
        client_id: u32,
        response: Sender<ConnectionState>
    },
}
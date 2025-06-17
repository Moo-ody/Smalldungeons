use crate::net::packets::packet_registry::ClientBoundPacket;
use crate::server::player::ClientId;

pub enum NetworkMessage {
    SendPacket {
        client_id: ClientId,
        packet: ClientBoundPacket
    },
    DisconnectClient {
        client_id: ClientId,
    },
}
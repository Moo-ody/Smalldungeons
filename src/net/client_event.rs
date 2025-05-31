use crate::net::packets::packet_registry::ServerBoundPackets;
use crate::server::player::ClientId;

pub enum ClientEvent {
    PacketReceived {
        client_id: ClientId,
        packet: ServerBoundPackets
    },
    NewPlayer {
        client_id: ClientId,
    },
    ClientDisconnected {
        client_id: ClientId,
    }
}
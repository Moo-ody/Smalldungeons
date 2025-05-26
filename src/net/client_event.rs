use crate::net::packets::packet_registry::ServerBoundPackets;

pub enum ClientEvent {
    PacketReceived {
        client_id: u32,
        packet: ServerBoundPackets
    },
    NewPlayer {
        client_id: u32,
    },
    ClientDisconnected {
        client_id: u32,
    }
}
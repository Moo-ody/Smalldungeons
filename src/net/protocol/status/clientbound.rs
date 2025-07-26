use crate::net::packets::packet::IdentifiedPacket;
use crate::net::packets::packet_serialize::PacketSerializable;
use crate::register_packets;
use blocks::packet_serializable;

register_packets! {
    StatusResponse = 0x00;
    StatusPong = 0x01;
}

packet_serializable! {
    pub struct StatusResponse {
        pub status: String,
    }
}

packet_serializable! {
    pub struct StatusPong {
        pub client_time: i64,
    }
}
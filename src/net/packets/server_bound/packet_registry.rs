use crate::net::connection_state::ConnectionState;
use crate::net::packets::server_bound::handshake::Handshake;
use crate::net::packets::server_bound::keep_alive::KeepAlive;
use crate::net::packets::server_bound::login_start::LoginStart;
use crate::net::packets::server_bound::ping::Ping;
use crate::net::packets::server_bound::status_request::StatusRequest;
use crate::register_serverbound_packets;

register_serverbound_packets! {
    ConnectionState::Handshaking {
        0x00 => Handshake,
    },
    ConnectionState::Play {
        0x00 => KeepAlive,
    },
    ConnectionState::Status {
        0x00 => StatusRequest,
        0x01 => Ping,
    },
    ConnectionState::Login {
        0x00 => LoginStart,
    },
}
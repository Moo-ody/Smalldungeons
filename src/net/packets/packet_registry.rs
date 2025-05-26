use crate::net::packets::client_bound::chunk_data::ChunkData;
use crate::net::packets::client_bound::join_game::JoinGame;
use crate::net::packets::client_bound::login_success::LoginSuccess;
use crate::net::packets::client_bound::pong::Pong;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::client_bound::server_info::ServerInfo;
use crate::{register_clientbound_packets, register_serverbound_packets};
use crate::net::connection_state::ConnectionState;
use crate::net::packets::client_bound::keep_alive::KeepAlive as CBKeepAlive;
use crate::net::packets::server_bound::handshake::Handshake;
use crate::net::packets::server_bound::keep_alive::KeepAlive as SBKeepAlive;
use crate::net::packets::server_bound::login_start::LoginStart;
use crate::net::packets::server_bound::ping::Ping;
use crate::net::packets::server_bound::player_look::PlayerLook;
use crate::net::packets::server_bound::player_pos_look::PlayerPosLook;
use crate::net::packets::server_bound::player_position::PlayerPosition;
use crate::net::packets::server_bound::status_request::StatusRequest;

register_clientbound_packets! {
    JoinGame,
    LoginSuccess,
    Pong,
    PositionLook,
    ServerInfo,
    ChunkData,
    CBKeepAlive,
}

register_serverbound_packets! {
    ConnectionState::Handshaking {
        0x00 => Handshake,
    },
    ConnectionState::Play {
        0x00 => SBKeepAlive,
        0x04 => PlayerPosition,
        0x05 => PlayerLook,
        0x06 => PlayerPosLook,
    },
    ConnectionState::Status {
        0x00 => StatusRequest,
        0x01 => Ping,
    },
    ConnectionState::Login {
        0x00 => LoginStart,
    },
}
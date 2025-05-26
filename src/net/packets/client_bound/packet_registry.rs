use crate::net::packets::client_bound::chunk_data::ChunkData;
use crate::net::packets::client_bound::join_game::JoinGame;
use crate::net::packets::client_bound::login_success::LoginSuccess;
use crate::net::packets::client_bound::pong::Pong;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::client_bound::server_info::ServerInfo;
use crate::register_clientbound_packets;

register_clientbound_packets! {
    JoinGame,
    LoginSuccess,
    Pong,
    PositionLook,
    ServerInfo,
    ChunkData,
}
#[derive(Debug, Clone)]
pub enum ConnectionState {
    Handshaking,
    Play,
    Status,
    Login,
}

pub fn get_state_from_id(id: i32) -> ConnectionState {
    match id {
        -1 => ConnectionState::Handshaking,
        0 => ConnectionState::Play,
        1 => ConnectionState::Status,
        2 => ConnectionState::Login,
        _ => panic!("Invalid connection state id: {}", id),
    }
}
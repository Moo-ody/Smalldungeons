use crate::net::connection_state::ConnectionState;
use crate::net::packets::client_bound::block_change::BlockChange;
use crate::net::packets::client_bound::chat::Chat;
use crate::net::packets::client_bound::chunk_data::ChunkData;
use crate::net::packets::client_bound::confirm_transaction::ConfirmTransaction as CBConfirmTransaction;
use crate::net::packets::client_bound::custom_payload::CustomPayload;
use crate::net::packets::client_bound::disconnect::Disconnect;
use crate::net::packets::client_bound::display_scoreboard::DisplayScoreboard;
use crate::net::packets::client_bound::entity::destroy_entities::DestroyEntities;
use crate::net::packets::client_bound::entity::entity_effect::EntityEffect;
use crate::net::packets::client_bound::entity::entity_head_look::EntityHeadLook;
use crate::net::packets::client_bound::entity::entity_look::EntityLook;
use crate::net::packets::client_bound::entity::entity_look_move::EntityLookMove;
use crate::net::packets::client_bound::entity::entity_metadata::EntityMetadata;
use crate::net::packets::client_bound::entity::entity_rel_move::EntityRelMove;
use crate::net::packets::client_bound::entity::entity_teleport::EntityTeleport;
use crate::net::packets::client_bound::entity::entity_velocity::EntityVelocity;
use crate::net::packets::client_bound::join_game::JoinGame;
use crate::net::packets::client_bound::keep_alive::KeepAlive as CBKeepAlive;
use crate::net::packets::client_bound::login_success::LoginSuccess;
use crate::net::packets::client_bound::particles::Particles;
use crate::net::packets::client_bound::player_list_header_footer::PlayerListHeaderFooter;
use crate::net::packets::client_bound::player_list_item::PlayerListItem;
use crate::net::packets::client_bound::pong::Pong;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::client_bound::scoreboard_objective::ScoreboardObjective;
use crate::net::packets::client_bound::server_info::ServerInfo;
use crate::net::packets::client_bound::set_slot::SetSlot;
use crate::net::packets::client_bound::sound_effect::SoundEffect;
use crate::net::packets::client_bound::spawn_mob::SpawnMob;
use crate::net::packets::client_bound::teams::Teams;
use crate::net::packets::client_bound::update_score::UpdateScore;
use crate::net::packets::client_bound::window_items::WindowItems;
use crate::net::packets::server_bound::chat_message::ChatMessage;
use crate::net::packets::server_bound::click_window::ClickWindow;
use crate::net::packets::server_bound::client_settings::ClientSettings;
use crate::net::packets::server_bound::confirm_transaction::ConfirmTransaction as SBConfirmTransaction;
use crate::net::packets::server_bound::handshake::Handshake;
use crate::net::packets::server_bound::held_item_change::HeldItemChange;
use crate::net::packets::server_bound::keep_alive::KeepAlive as SBKeepAlive;
use crate::net::packets::server_bound::login_start::LoginStart;
use crate::net::packets::server_bound::ping::Ping;
use crate::net::packets::server_bound::player_action::PlayerAction;
use crate::net::packets::server_bound::player_block_placement::PlayerBlockPlacement;
use crate::net::packets::server_bound::player_digging::PlayerDigging;
use crate::net::packets::server_bound::player_look::PlayerLook;
use crate::net::packets::server_bound::player_pos_look::PlayerPosLook;
use crate::net::packets::server_bound::player_position::PlayerPosition;
use crate::net::packets::server_bound::player_update::PlayerUpdate;
use crate::net::packets::server_bound::status_request::StatusRequest;
use crate::net::packets::server_bound::swing_animation::SwingAnimation;
use crate::{register_clientbound_packets, register_serverbound_packets};

register_clientbound_packets! {
    JoinGame,
    LoginSuccess,
    Pong,
    PositionLook,
    ServerInfo,
    ChunkData,
    CBKeepAlive,
    CBConfirmTransaction,
    Disconnect,
    SpawnMob,
    DestroyEntities,
    EntityVelocity,
    SetSlot,
    WindowItems,
    Chat,
    BlockChange,
    CustomPayload,

    EntityLookMove,
    EntityHeadLook,
    EntityMetadata,
    EntityRelMove,
    EntityLook,
    EntityTeleport,
    EntityEffect,
    
    SoundEffect,
    Particles,
    
    PlayerListItem,
    PlayerListHeaderFooter,
    ScoreboardObjective,
    UpdateScore,
    DisplayScoreboard,
    Teams,
    
}

register_serverbound_packets! {
    ConnectionState::Handshaking {
        0x00 => Handshake,
    },
    ConnectionState::Play {
        0x00 => SBKeepAlive,
        0x01 => ChatMessage,
        0x03 => PlayerUpdate,
        0x04 => PlayerPosition,
        0x05 => PlayerLook,
        0x06 => PlayerPosLook,
        0x0B => PlayerAction,
        0x0A => SwingAnimation,
        0x08 => PlayerBlockPlacement,
        0x09 => HeldItemChange,
        0x0e => ClickWindow,
        0x07 => PlayerDigging,
        0x15 => ClientSettings,
        0x0F => SBConfirmTransaction,
    },
    ConnectionState::Status {
        0x00 => StatusRequest,
        0x01 => Ping,
    },
    ConnectionState::Login {
        0x00 => LoginStart,
    },
}
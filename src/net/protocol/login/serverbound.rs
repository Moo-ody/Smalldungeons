use crate::net::client::Client;
use crate::net::connection_state::ConnectionState;
use crate::net::internal_packets::MainThreadMessage;
use crate::net::packets::packet::{ProcessContext, ProcessPacket};
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::login::clientbound::LoginSuccess;
use crate::register_serverbound_packets;
use crate::server::player::player::GameProfile;
use crate::server::utils::sized_string::SizedString;
use blocks::packet_deserializable;
use std::collections::HashMap;
use uuid::Uuid;

register_serverbound_packets! {
    Login;
    LoginStart = 0x00;
    // EncryptionResponse = 0x01,
}

packet_deserializable! {
    pub struct LoginStart {
        pub username: SizedString<16>
    }
}

impl ProcessPacket for LoginStart {
    async fn process<'a>(&self, client: &mut Client, context: ProcessContext<'a>) -> anyhow::Result<()> {
        println!("player {} attempted to join", self.username.as_str());
        let mut packet_buffer = PacketBuffer::new();

        let game_profile = GameProfile {
            uuid: Uuid::parse_str("d74cb748-b23b-4a99-b41e-b85f73d41999")?,
            username: self.username.0.to_string(),
            properties: HashMap::new(),
        };

        packet_buffer.write_packet(&LoginSuccess {
            uuid: game_profile.uuid.to_string(),
            name: game_profile.username.to_string(),
        });
        context.network_thread_tx.send(packet_buffer.get_packet_message(&client.client_id))?;
        context.main_thread_tx.send(MainThreadMessage::NewPlayer {
            client_id: client.client_id,
            profile: game_profile
        })?;
        client.connection_state = ConnectionState::Play;
        Ok(())
    }
}
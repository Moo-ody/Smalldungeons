use crate::net::client::Client;
use crate::net::packets::packet::{ProcessContext, ProcessPacket};
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::status::clientbound::{StatusPong, StatusResponse};
use crate::register_serverbound_packets;
use base64::engine::general_purpose;
use base64::Engine;
use blocks::packet_deserializable;
use once_cell::sync::Lazy;

register_serverbound_packets! {
    Status;
    StatusRequest = 0x00;
    StatusPing = 0x01;
}

packet_deserializable! {
    pub struct StatusRequest;
}

// not real sure where to put this, but here should be fine for now.
const FAVICON_BYTES: &[u8] = include_bytes!("../../../assets/favicon.png");

pub static STATUS_RESPONSE_JSON: Lazy<String> = Lazy::new(|| {
    let encoded_image = general_purpose::STANDARD.encode(FAVICON_BYTES);
    let version = env!("CARGO_PKG_VERSION");

    format!(r#"{{
        "version": {{ "name": "1.8.9", "protocol": 47 }},
        "players": {{ "max": 1, "online": 0 }},
        "description": {{ "text": "RustClear", "color": "gold", "extra": [{{ "text": " version ", "color": "gray" }}, {{ "text": "{version}", "color": "green" }}] }},
        "favicon": "data:image/png;base64,{encoded_image}"
    }}"#)
});

impl ProcessPacket for StatusRequest {
    async fn process<'a>(&self, client: &mut Client, context: ProcessContext<'a>) -> anyhow::Result<()> {
        let mut packet_buffer = PacketBuffer::new();
        packet_buffer.write_packet(&StatusResponse {
            status: STATUS_RESPONSE_JSON.parse()?,
        });
        context.network_thread_tx.send(packet_buffer.get_packet_message(&client.client_id))?;
        Ok(())
    }
}

packet_deserializable! {
    pub struct StatusPing {
        pub client_time: i64,
    }
}

impl ProcessPacket for StatusPing {
    async fn process<'a>(&self, client: &mut Client, context: ProcessContext<'a>) -> anyhow::Result<()> {
        let mut packet_buffer = PacketBuffer::new();
        packet_buffer.write_packet(&StatusPong {
            client_time: self.client_time,
        });
        context.network_thread_tx.send(packet_buffer.get_packet_message(&client.client_id))?;
        Ok(())
    }
}


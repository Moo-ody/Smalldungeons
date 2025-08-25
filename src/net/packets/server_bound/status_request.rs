use crate::net::packets::client_bound::server_info::ServerInfo;
use crate::net::packets::packet::{SendPacket, ServerBoundPacket};
use crate::net::packets::packet_context::PacketContext;
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use bytes::BytesMut;
use once_cell::sync::Lazy;

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

#[derive(Debug)]
pub struct StatusRequest;

#[async_trait::async_trait]
impl ServerBoundPacket for StatusRequest {
    async fn read_from(buf: &mut BytesMut) -> Result<Self> {
        Ok(Self)
    }

    async fn process<'a>(&self, context: PacketContext<'a>) -> Result<()> {
        ServerInfo {
            status: STATUS_RESPONSE_JSON.to_string(),
        }.send_packet(context.client.client_id(), context.network_tx)?;
        Ok(())
    }
}
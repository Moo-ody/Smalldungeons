use tokio::sync::mpsc::UnboundedSender;
use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;

#[derive(Clone)]
pub struct PacketContext {
    pub client_id: u32,
    pub network_tx: UnboundedSender<NetworkMessage>,
    pub event_tx: UnboundedSender<ClientEvent>,
}
use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::server::player::ClientId;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct PacketContext {
    pub client_id: ClientId,
    pub network_tx: UnboundedSender<NetworkMessage>,
    pub event_tx: UnboundedSender<ClientEvent>,
}
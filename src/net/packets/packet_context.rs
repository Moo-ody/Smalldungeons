use crate::net::client::Client;
use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use tokio::sync::mpsc::UnboundedSender;

pub struct PacketContext<'a> {
    pub client: &'a mut Client,
    pub network_tx: UnboundedSender<NetworkMessage>,
    pub event_tx: UnboundedSender<ClientEvent>,
}
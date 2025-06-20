use crate::net::client::Client;
use crate::net::internal_packets::{MainThreadMessage, NetworkThreadMessage};
use tokio::sync::mpsc::UnboundedSender;

pub struct PacketContext<'a> {
    pub client: &'a mut Client,
    pub network_tx: &'a UnboundedSender<NetworkThreadMessage>,
    pub main_tx: &'a UnboundedSender<MainThreadMessage>,
}
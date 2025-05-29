use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::SendPacket;
use crate::server::entity::entity::Entity;
use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

// here should be like custom player,
// that handles player interaction and all kinds of stuff
pub struct Player {
    pub client_id: u32,
    pub entity: Entity
}

impl Player {

    pub fn set_position(
        &mut self,
        network_tx: &UnboundedSender<NetworkMessage>,
        x: f64,
        y: f64,
        z: f64,
    ) -> Result<()> {
        self.entity.update_position(x, y, z);
        PositionLook::from_player(&self).send_packet(self.client_id, network_tx)?;
        Ok(())
    }

}
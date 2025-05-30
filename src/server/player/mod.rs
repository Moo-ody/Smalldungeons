use crate::server::entity::entity::Entity;
use crate::server::world::World;
use anyhow::Result;

// here should be like custom player,
// that handles player interaction and all kinds of stuff
#[derive(Debug)]
pub struct Player {
    pub client_id: u32,
    pub entity_id: i32,

    pub last_keep_alive: i32,
    pub ping: i32
}

impl Player {
    pub fn new(client_id: u32, entity_id: i32) -> Self {
        Self {
            client_id,
            entity_id,
            last_keep_alive: -1,
            ping: -1,
        }
    }

    pub fn get_entity<'a>(&self, world: &'a mut World) -> Result<&'a mut Entity> {
        world.entities.get_mut(&self.entity_id).ok_or_else(|| anyhow::anyhow!("Couldn't find corresponding entity for {self:?}"))
    }

    // pub fn set_position(
    //     &mut self,
    //     network_tx: &UnboundedSender<NetworkMessage>,
    //     x: f64,
    //     y: f64,
    //     z: f64,
    // ) -> Result<()> {
    //     self.entity.update_position(x, y, z);
    //     PositionLook::from_player(&self).send_packet(self.client_id, network_tx)?;
    //     Ok(())
    // }

}
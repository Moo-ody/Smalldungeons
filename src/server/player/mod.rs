pub mod inventory;

use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::spawn_mob::SpawnMob;
use crate::net::packets::packet::SendPacket;
use crate::server::entity::entity::{Entity, EntityId};
use crate::server::player::inventory::{Inventory, ItemSlot};
use crate::server::server::Server;
use crate::server::utils::scoreboard::scoreboard::Scoreboard;
use crate::server::world::World;
use anyhow::{bail, Result};
use std::collections::HashSet;
use tokio::sync::mpsc::UnboundedSender;

/// type alias to represent a client's user id.
///
/// alias for a u32
pub type ClientId = u32;

// here should be like custom player,
// that handles player interaction and all kinds of stuff
#[derive(Debug)]
pub struct Player {
    pub server: *mut Server,

    pub client_id: ClientId,
    pub entity_id: EntityId,

    pub scoreboard: Scoreboard,

    pub last_keep_alive: i32,
    pub ping: i32,

    pub is_sneaking: bool,

    pub inventory: Inventory,
    pub held_slot: u8,

    pub observed_entities: HashSet<EntityId>,
}

impl Player {
    pub fn new(server: &mut Server, client_id: ClientId, entity_id: EntityId) -> Self {
        Self {
            server,
            client_id,
            entity_id,
            scoreboard: Scoreboard::new("§e§lSKYBLOCK"),
            last_keep_alive: -1,
            ping: -1,
            is_sneaking: false,
            inventory: Inventory::empty(),
            held_slot: 0,
            observed_entities: HashSet::new(),
        }
    }

    // potentially unsafe
    pub fn server_mut<'a>(&self) -> &'a mut Server {
        unsafe { self.server.as_mut().expect("Server is null") }
    }

    pub fn get_entity<'a>(&self, world: &'a World) -> Result<&'a Entity> {
        world.entities.get(&self.entity_id).ok_or_else(|| anyhow::anyhow!("Couldn't find corresponding entity for {self:?}"))
    }

    pub fn get_entity_mut<'a>(&self, world: &'a mut World) -> Result<&'a mut Entity> {
        world.entities.get_mut(&self.entity_id).ok_or_else(|| anyhow::anyhow!("Couldn't find corresponding entity for {self:?}"))
    }

    /// function to have a player start observing an entity
    ///
    /// presumably to be called when the entity should be loaded for said player.
    /// the player will be added to the entities observing list
    /// and the entity to the players observing list. (the latter is for removing themselves from the entities list if the entity should be unloaded for them)
    pub fn observe_entity(&mut self, entity: &mut Entity, network_tx: &UnboundedSender<NetworkMessage>) -> Result<()> {
        if self.entity_id == entity.entity_id { bail!("Can't observe self") }
        self.observed_entities.insert(entity.entity_id);
        entity.observing_players.insert(self.client_id);

        // todo: logic to get the right packet for the entity type. Ie: SpawnObject for objects, SpawnPlayer for players, etc.
        SpawnMob::from_entity(entity)?.send_packet(self.client_id, network_tx)?;
        Ok(())
    }

    /// function to have a player stop observing an entity
    ///
    /// presumably called when the entity should be unloaded for said player.
    ///
    /// todo: handling for destroy entities packet sending.
    pub fn stop_observing_entity(&mut self, entity: &mut Entity) {
        self.observed_entities.remove(&entity.entity_id);
        entity.observing_players.remove(&self.client_id);
    }

    pub fn handle_right_click(&self) {
        if let Some(ItemSlot::Filled(item, _)) = self.inventory.get_hotbar_slot(self.held_slot as usize) {
            item.on_right_click(self).unwrap()
        }
    }

}
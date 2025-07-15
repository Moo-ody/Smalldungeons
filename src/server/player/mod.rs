pub mod inventory;
pub mod ui;
pub mod scoreboard;

use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::packets::client_bound::chat::Chat;
use crate::net::packets::client_bound::close_window::CloseWindowPacket;
use crate::net::packets::client_bound::open_window::{InventoryType, OpenWindowPacket};
use crate::net::packets::client_bound::set_slot::SetSlot;
use crate::net::packets::client_bound::spawn_mob::SpawnMob;
use crate::net::packets::client_bound::window_items::WindowItems;
use crate::net::packets::packet::SendPacket;
use crate::server::entity::entity::{Entity, EntityId};
use crate::server::player::inventory::{Inventory, ItemSlot};
use crate::server::player::scoreboard::Scoreboard;
use crate::server::player::ui::UI;
use crate::server::server::Server;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
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
    pub network_tx: UnboundedSender<NetworkThreadMessage>,
    
    pub username: String,
    pub client_id: ClientId,
    pub entity_id: EntityId,

    pub scoreboard: Scoreboard,

    pub last_keep_alive: i32,
    pub ping: i32,

    pub is_sneaking: bool,
    
    pub current_window_id: u8,
    pub current_ui: UI,

    pub inventory: Inventory,
    pub held_slot: u8,

    // im assuming this is for sending packets for entities inside of player's render distance.
    // if so, it'd be better to just loop over every player and check if in distance instead
    // because:
    // - there is usually (and should) only be 1 player at a time
    // - most entities would be in render distance already
    // obviously this would be better if the render distance was much larger and much more entities
    pub observed_entities: HashSet<EntityId>,
}

impl Player{
    
    pub fn new(server: &mut Server, username: String, client_id: ClientId, entity_id: EntityId) -> Self {
        Self {
            server,
            network_tx: server.network_tx.clone(),
            username,
            client_id,
            entity_id,
            scoreboard: Scoreboard::new(),
            last_keep_alive: -1,
            ping: -1,
            is_sneaking: false,
            current_window_id: 1,
            current_ui: UI::None,
            
            inventory: Inventory::empty(),
            held_slot: 0,
            observed_entities: HashSet::new(),
        }
    }
    
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
    pub fn observe_entity(&mut self, entity: &mut Entity, network_tx: &UnboundedSender<NetworkThreadMessage>) -> Result<()> {
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

    pub fn handle_left_click(&mut self) {
        
    }
    
    /// todo: better interaction, maybe?
    pub fn handle_right_click(&mut self) {
        if let Some(ItemSlot::Filled(item)) = self.inventory.get_hotbar_slot(self.held_slot as usize) {
            item.on_right_click(self).unwrap()
        }
    }
    
    pub fn open_ui(&mut self, ui: UI) -> Result<()> {
        self.current_ui = ui;
        
        if let Some(container_data) = ui.get_container_data() {
            self.current_window_id += 1;
            
            OpenWindowPacket {
                window_id: self.current_window_id,
                inventory_type: InventoryType::Container,
                window_title: ChatComponentTextBuilder::new(container_data.title).build(),
                slot_count: container_data.slot_amount,
            }.send_packet(self.client_id, &self.server_mut().network_tx)?;
            
            self.sync_inventory()?;
        }
        Ok(()) 
    }
    
    pub fn close_ui(&mut self) -> Result<()> {
        self.current_ui = UI::None;
        CloseWindowPacket {
            window_id: self.current_window_id as i8,
        }.send_packet(self.client_id, &self.server_mut().network_tx)?;
        Ok(())
    }

    
    pub fn sync_inventory(&mut self) -> Result<()> {
        let network_tx = &self.server_mut().network_tx;
        let mut inv_items = Vec::new();
        
        for item in &self.inventory.items {
            inv_items.push(item.get_item_stack())
        }
        
        if let Some(items) = self.current_ui.get_container_contents(self.server_mut(), &self.client_id)  {
            WindowItems {
                window_id: self.current_window_id,
                items,
            }.send_packet(self.client_id, &self.server_mut().network_tx)?;
        }
        
        WindowItems {
            window_id: 0,
            items: inv_items,
        }.send_packet(self.client_id, network_tx)?;
        
        if let UI::Inventory = self.current_ui { 
            SetSlot {
                window_id: -1,
                slot: 0,
                item_stack: self.inventory.dragged_item.get_item_stack(),
            }.send_packet(self.client_id, network_tx)?;
        } else {
            SetSlot {
                window_id: -1,
                slot: 0,
                item_stack: None,
            }.send_packet(self.client_id, network_tx)?;
        }
        Ok(())
    }
    
    pub fn send_msg(&self, msg: &str) -> Result<()> {
        Chat {
            component: ChatComponentTextBuilder::new(msg).build(),
            typ: 0,
        }.send_packet(self.client_id, &self.network_tx)
    }
}
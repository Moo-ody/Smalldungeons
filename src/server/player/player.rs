use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::packets::packet::IdentifiedPacket;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::packets::packet_serialize::PacketSerializable;
use crate::net::protocol::play::clientbound::{Chat, OpenWindow, SetSlot, WindowItems};
use crate::server::entity::entity::EntityId;
use crate::server::player::container_ui::UI;
use crate::server::player::inventory::{Inventory, ItemSlot};
use crate::server::player::scoreboard::Scoreboard;
use crate::server::server::Server;
use crate::server::utils::aabb::AABB;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use tokio::sync::mpsc::UnboundedSender;

/// type alias to represent a client's user id.
///
/// alias for a u32
pub type ClientId = u32;

// add uuid
#[derive(Debug)]
pub struct GameProfile {
    pub username: String,
}

#[derive(Debug)]
pub struct Player {
    pub server: *mut Server,
    pub packet_buffer: PacketBuffer,
    pub network_tx: UnboundedSender<NetworkThreadMessage>,
    
    pub profile: GameProfile,
    pub client_id: ClientId,
    pub entity_id: EntityId,
    
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    
    pub last_position: DVec3,   
    pub last_yaw: f32,
    pub last_pitch: f32,

    pub ticks_existed: u32,
    pub last_keep_alive: i32,
    pub ping: i32,

    pub is_sneaking: bool,

    pub inventory: Inventory,
    pub held_slot: u8,
    
    pub window_id: i8,
    pub current_ui: UI,
    // pub current_ui: UI,

    pub sidebar: Scoreboard,
}

impl Player {
    
    pub fn new(
        server: &mut Server,
        client_id: ClientId,
        profile: GameProfile,
        position: DVec3,
    ) -> Self {
        Self {
            server,
            packet_buffer: PacketBuffer::new(),
            network_tx: server.network_tx.clone(),
            profile,
            client_id,
            entity_id: server.world.new_entity_id(),

            position,
            on_ground: false,
            yaw: 0.0,
            pitch: 0.0,
            last_position: DVec3::ZERO,
            last_yaw: 0.0,
            last_pitch: 0.0,

            ticks_existed: 0,
            last_keep_alive: -1,
            ping: -1,
            is_sneaking: false,

            inventory: Inventory::empty(),
            held_slot: 0,
            
            window_id: 1,
            current_ui: UI::None,
            
            sidebar: Scoreboard::new(),
            
            // observed_entities: HashSet::new(),
        }
    }

    /// gets a reference to the server
    pub fn server_mut<'a>(&self) -> &'a mut Server {
        unsafe { self.server.as_mut().expect("Server is null") }
    }

    /// gets a reference to the world
    pub fn world_mut<'a>(&self) -> &'a mut World {
        &mut self.server_mut().world
    }
    
    // /// sends a packet to the player
    // pub fn send_packet<T>(&self, packet: impl SendPacket<T>) -> anyhow::Result<()> {
    //     packet.send_packet(self.client_id, &self.network_tx)
    // }
    
    pub fn write_packet<P : IdentifiedPacket + PacketSerializable>(&mut self, packet: &P) {
        self.packet_buffer.write_packet(packet);
    }
    
    pub fn flush_packets(&mut self) {
        if self.packet_buffer.buffer.len() != 0 {
            let result = self.network_tx.send(self.packet_buffer.get_packet_message(&self.client_id));
            if result.is_err() { 
                panic!("error happened flushing packets");
            }
        }
    }
    
    // todo: tick function here?, 
    // pub fn tick(&mut self) -> anyhow::Result<()> {
    //     self.ticks_existed += 1;
    //     self.send_packet(ConfirmTransaction::new())?;
    //     Ok(())
    // }
    
    /// updates player position and sets last position
    pub fn set_position(&mut self, x: f64, y: f64, z: f64) {
        self.last_position = self.position;
        self.position = DVec3::new(x, y, z);
    }
    
    pub fn collision_aabb(&self) -> AABB {
        let w = 0.3;
        let h = 1.8;
        AABB::new(
            DVec3::new(self.position.x - w, self.position.y, self.position.z - w),
            DVec3::new(self.position.x + w, self.position.y + h, self.position.z + w),
        )
    }

    // /// function to have a player start observing an entity
    // ///
    // /// presumably to be called when the entity should be loaded for said player.
    // /// the player will be added to the entities observing list
    // /// and the entity to the players observing list. (the latter is for removing themselves from the entities list if the entity should be unloaded for them)
    // pub fn observe_entity(&mut self, entity: &mut Entity, network_tx: &UnboundedSender<NetworkThreadMessage>) -> anyhow::Result<()> {
    //     if self.entity_id == entity.entity_id { bail!("Can't observe self") }
    //     self.observed_entities.insert(entity.entity_id);
    //     entity.observing_players.insert(self.client_id);
    // 
    //     // todo: logic to get the right packet for the entity type. Ie: SpawnObject for objects, SpawnPlayer for players, etc.
    //     SpawnMob::from_entity(entity)?.send_packet(self.client_id, network_tx)?;
    //     Ok(())
    // }

    // /// function to have a player stop observing an entity
    // ///
    // /// presumably called when the entity should be unloaded for said player.
    // ///
    // /// todo: handling for destroy entities packet sending.
    // pub fn stop_observing_entity(&mut self, entity: &mut Entity) {
    //     self.observed_entities.remove(&entity.entity_id);
    //     entity.observing_players.remove(&self.client_id);
    // }

    pub fn handle_left_click(&mut self) {
        
    }
    
    /// todo: better interaction, maybe?
    pub fn handle_right_click(&mut self) {
        if let Some(ItemSlot::Filled(item)) = self.inventory.get_hotbar_slot(self.held_slot as usize) {
            item.on_right_click(self).unwrap()
        }
    }
    
    pub fn open_ui(&mut self, ui: UI) {
        self.current_ui = ui;
        // kind of temporary solution,
        // instead of just putting the item in an available slot if it is dragged
        if ui == UI::Inventory { 
            if let ItemSlot::Filled(item) = self.inventory.dragged_item { 
                self.write_packet(&SetSlot {
                    window_id: -1,
                    slot: 0,
                    item_stack: Some(item.get_item_stack()),
                })
            }
        }
        if let Some(container_data) = ui.get_container_data() {
            self.window_id += 1;
            self.write_packet(&OpenWindow {
                window_id: self.window_id,
                inventory_type: "minecraft:container".into(),
                window_title: ChatComponentTextBuilder::new(container_data.title).build(),
                slot_count: container_data.slot_amount,
            });
            self.sync_inventory();
        }
    }
    
    pub fn sync_inventory(&mut self) {
        let mut inv_items = Vec::new();
        
        for item in &self.inventory.items {
            inv_items.push(item.get_item_stack())
        }
        
        if let Some(items) = self.current_ui.get_container_contents(self.server_mut(), &self.client_id)  {
            self.write_packet(&WindowItems {
                window_id: self.window_id,
                items,
            });
        }
        
        self.write_packet(&WindowItems {
            window_id: 0,
            items: inv_items,
        });
        if let UI::Inventory = self.current_ui {
            self.write_packet(&SetSlot {
                window_id: -1,
                slot: 0,
                item_stack: self.inventory.dragged_item.get_item_stack(),
            })
        } else {
            self.write_packet(&SetSlot {
                window_id: -1,
                slot: 0,
                item_stack: None,
            })
        }
    }
    
    pub fn send_message(&mut self, msg: &str) {
        self.write_packet(&Chat {
            component: ChatComponentTextBuilder::new(msg).build(),
            chat_type: 0 
        })
    }
}
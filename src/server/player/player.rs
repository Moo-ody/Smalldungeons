use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::packets::client_bound::chat::Chat;
use crate::net::packets::client_bound::close_window::CloseWindowPacket;
use crate::net::packets::client_bound::open_window::{InventoryType, OpenWindowPacket};
use crate::net::packets::client_bound::set_slot::SetSlot;
use crate::net::packets::client_bound::window_items::WindowItems;
use crate::net::packets::packet::SendPacket;
use crate::server::entity::entity::EntityId;
use crate::server::player::inventory::{Inventory, ItemSlot};
use crate::server::player::scoreboard::Scoreboard;
use crate::server::player::ui::UI;
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

    // Teleport system fields
    pub home_position: Option<DVec3>,
    pub home_direction: Option<(f32, f32)>, // (yaw, pitch)

    pub inventory: Inventory,
    pub held_slot: u8,
    
    pub current_window_id: u8,
    pub current_ui: UI,

    pub sidebar: Scoreboard,
    
    // im assuming this is for sending packets for entities inside of player's render distance.
    // if so, it'd be better to just loop over every player and check if in distance instead
    // because:
    // - there is usually (and should) only be 1 player at a time
    // - most entities would be in render distance already
    // obviously this would be better if the render distance was much larger and much more entities
    // pub observed_entities: HashSet<EntityId>,
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

            // Teleport system fields
            home_position: None,
            home_direction: None,

            inventory: Inventory::empty(),
            held_slot: 0,
            
            current_window_id: 1,
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
    
    /// sends a packet to the player
    pub fn send_packet<T>(&self, packet: impl SendPacket<T>) -> anyhow::Result<()> {
        packet.send_packet(self.client_id, &self.network_tx)
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

    /// Check if player would intersect blocks at a specific position
    pub fn is_intersecting_blocks_at(&self, x: f64, y: f64, z: f64) -> bool {
        let world = &self.server_mut().world;
        
        // Check feet position (Y) and head position (Y + 1.62)
        let feet_y = y.floor() as i32;
        let head_y = (y + 1.62).floor() as i32;
        let block_x = x.floor() as i32;
        let block_z = z.floor() as i32;

        // Check if both feet and head positions are blocked
        let feet_block = world.get_block_at(block_x, feet_y, block_z);
        let head_block = world.get_block_at(block_x, head_y, block_z);

        // If either position is not passable, player is intersecting
        !is_block_passable_for_player(feet_block) || !is_block_passable_for_player(head_block)
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
        if let Some(ItemSlot::Filled(item)) = self.inventory.get_hotbar_slot(self.held_slot as usize) {
            let _ = item.on_left_click(self);
        }
    }
    
    /// todo: better interaction, maybe?
    pub fn handle_right_click(&mut self) {
        if let Some(ItemSlot::Filled(item)) = self.inventory.get_hotbar_slot(self.held_slot as usize) {
            item.on_right_click(self).unwrap()
        }
    }
    
    pub fn open_ui(&mut self, ui: UI) -> anyhow::Result<()> {
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
    
    pub fn close_ui(&mut self) -> anyhow::Result<()> {
        self.current_ui = UI::None;
        CloseWindowPacket {
            window_id: self.current_window_id as i8,
        }.send_packet(self.client_id, &self.server_mut().network_tx)?;
        Ok(())
    }

    /// Check if the player is currently in water
    pub fn is_in_water(&self) -> bool {
        let world = self.world_mut();
        let block_pos = (
            self.position.x.floor() as i32,
            self.position.y.floor() as i32,
            self.position.z.floor() as i32,
        );
        
        // Use the world's get_block_at method which handles chunk access properly
        let block = world.get_block_at(block_pos.0, block_pos.1, block_pos.2);
        
        matches!(block, 
            crate::server::block::blocks::Blocks::FlowingWater { .. } |
            crate::server::block::blocks::Blocks::StillWater { .. }
        )
    }

    /// Check if the player has Depth Strider boots equipped
    pub fn has_depth_strider(&self) -> bool {
        // Check if Depth Strider boots are in any inventory slot
        for slot in &self.inventory.items {
            if let crate::server::player::inventory::ItemSlot::Filled(item) = slot {
                if matches!(item, crate::server::items::Item::DepthStriderBoots) {
                    return true;
                }
            }
        }
        false
    }

    /// Apply Depth Strider effect if player is in water and has the boots
    pub fn apply_depth_strider_effect(&mut self) -> anyhow::Result<()> {
        let is_in_water = self.is_in_water();
        let has_depth_strider = self.has_depth_strider();
        
        let mut attributes = crate::server::player::attribute::AttributeMap::new();
        
        // Movement speed to match Hypixel Skyblock (22 bps walk, 28 bps sprint)
        // Base Minecraft walk: ~4.317 bps, sprint: ~5.612 bps
        // Target: walk ~22 bps, sprint ~28 bps
        // Multiplier needed: ~5.1x for walk, ~5.0x for sprint
        // Base Minecraft speed is 0.1, so we need 0.1 * 5.1 = 0.51
        attributes.insert(crate::server::player::attribute::Attribute::MovementSpeed, 0.51);
        
        self.send_packet(crate::net::packets::client_bound::entity::entity_properties::EntityProperties {
            entity_id: self.entity_id,
            properties: attributes,
        })?;
        
        Ok(())
    }

    
    pub fn sync_inventory(&mut self) -> anyhow::Result<()> {
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
    
    pub fn send_msg(&self, msg: &str) -> anyhow::Result<()> {
        Chat {
            component: ChatComponentTextBuilder::new(msg).build(),
            typ: 0,
        }.send_packet(self.client_id, &self.network_tx)
    }
}

/// Check if a block is passable for player movement
#[inline]
fn is_block_passable_for_player(block: crate::server::block::blocks::Blocks) -> bool {
    match block {
        crate::server::block::blocks::Blocks::Air
        | crate::server::block::blocks::Blocks::FlowingWater { .. }
        | crate::server::block::blocks::Blocks::StillWater { .. }
        | crate::server::block::blocks::Blocks::FlowingLava { .. }
        | crate::server::block::blocks::Blocks::Lava { .. }
        | crate::server::block::blocks::Blocks::Tallgrass { .. }
        | crate::server::block::blocks::Blocks::Deadbush
        | crate::server::block::blocks::Blocks::Torch { .. }
        | crate::server::block::blocks::Blocks::UnlitRedstoneTorch { .. }
        | crate::server::block::blocks::Blocks::RedstoneTorch { .. }
        | crate::server::block::blocks::Blocks::Redstone { .. }
        | crate::server::block::blocks::Blocks::YellowFlower
        | crate::server::block::blocks::Blocks::RedFlower { .. }
        | crate::server::block::blocks::Blocks::Vine { .. }
        | crate::server::block::blocks::Blocks::Fire
        | crate::server::block::blocks::Blocks::Lilypad
        | crate::server::block::blocks::Blocks::Carpet { .. }
        | crate::server::block::blocks::Blocks::SnowLayer { .. }
        | crate::server::block::blocks::Blocks::Skull { .. }
        | crate::server::block::blocks::Blocks::FlowerPot { .. }
        | crate::server::block::blocks::Blocks::RedstoneComparator { .. }
        | crate::server::block::blocks::Blocks::PoweredRedstoneComparator { .. }
        | crate::server::block::blocks::Blocks::RedstoneRepeater { .. }
        | crate::server::block::blocks::Blocks::PoweredRedstoneRepeater { .. }
        | crate::server::block::blocks::Blocks::Rail { .. }
        | crate::server::block::blocks::Blocks::PoweredRail { .. }
        | crate::server::block::blocks::Blocks::DetectorRail { .. }
        | crate::server::block::blocks::Blocks::DaylightSensor { .. }
        | crate::server::block::blocks::Blocks::InvertedDaylightSensor { .. }
        | crate::server::block::blocks::Blocks::Ladder { .. }
        | crate::server::block::blocks::Blocks::Trapdoor { open: true, .. }
        | crate::server::block::blocks::Blocks::IronTrapdoor { open: true, .. }
        | crate::server::block::blocks::Blocks::SpruceFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::BirchFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::JungleFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::DarkOakFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::AcaciaFenceGate { open: true, .. } => true,
        _ => false,
    }
}
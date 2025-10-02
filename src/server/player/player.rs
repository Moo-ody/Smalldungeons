use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::packets::packet::IdentifiedPacket;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::packets::packet_serialize::PacketSerializable;
use crate::net::protocol::play::clientbound::{Chat, OpenWindow, SetSlot, WindowItems};
use crate::server::entity::entity::EntityId;
use crate::server::player::container_ui::UI;
use crate::server::player::inventory::{Inventory, ItemSlot};
use crate::server::player::terminal::Terminal;
use crate::server::player::scoreboard::Scoreboard;
use crate::server::server::Server;
use crate::server::utils::aabb::AABB;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

/// type alias to represent a client's user id.
///
/// alias for a u32
pub type ClientId = u32;

// add uuid
#[derive(Debug, Clone)]
pub struct GameProfileProperty {
    pub value: String,
    pub signature: Option<String>
}

#[derive(Debug, Clone)]
pub struct GameProfile {
    pub uuid: Uuid,
    pub username: String,
    pub properties: HashMap<String, GameProfileProperty>
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
    pub current_terminal: Option<Terminal>,

    pub sidebar: Scoreboard,
    
    // Bonzo Staff cooldown tracking
    pub bonzo_last_shot_tick: u64,
    
    // Jerry-Chine Gun cooldown tracking
    pub jerry_last_shot_tick: u64,
    
    // Lava bounce tracking
    pub in_lava: bool,
    pub lava_bounce_last_tick: u64,
    pub lava_bounce_enabled: bool,
}

impl Player {
    
    pub fn new(
        server: &mut Server,
        client_id: ClientId,
        profile: GameProfile,
        position: DVec3,
        yaw: f32,
        pitch: f32,
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
            yaw,
            pitch,
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
            current_terminal: None,

            sidebar: Scoreboard::new(),
            
            // Bonzo Staff cooldown tracking
            bonzo_last_shot_tick: 0,
            
            // Jerry-Chine Gun cooldown tracking
            jerry_last_shot_tick: 0,
            
            // Lava bounce tracking
            in_lava: false,
            lava_bounce_last_tick: 0,
            lava_bounce_enabled: true, // Enable lava bounce by default
            
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
    
    pub fn write_packet<P : IdentifiedPacket + PacketSerializable>(&mut self, packet: &P) {
        self.packet_buffer.write_packet(packet);
    }
    
    pub fn flush_packets(&mut self) {
        if !self.packet_buffer.buffer.is_empty() {
            let _ = self.network_tx.send(self.packet_buffer.get_packet_message(&self.client_id));
        }
    }
    
    // todo: tick function here?, 
    // pub fn tick(&mut self) -> anyhow::Result<()> {
    //     self.ticks_existed += 1;
    //     self.send_packet(ConfirmTransaction::new())?;
    //     Ok(())
    // }
    
    /// updates player position
    pub fn set_position(&mut self, x: f64, y: f64, z: f64) {
        // self.last_position = self.position;
        self.position = DVec3::new(x, y, z);
        
        // Check for falling blocks collision
        self.check_fallingblocks_collision();
        
        // Check for lava bounce
        self.check_lava_bounce();
    }
    
    /// Check for lava bounce when player enters lava
    fn check_lava_bounce(&mut self) {
        use crate::server::block::blocks::Blocks;
        use crate::net::protocol::play::clientbound::EntityVelocity;
        use crate::net::var_int::VarInt;
        
        let world = self.world_mut();
        let block_pos = DVec3::new(
            self.position.x.floor(),
            self.position.y.floor(),
            self.position.z.floor(),
        );
        
        // Check if player's feet are touching lava surface (at floor level)
        let feet_block = world.get_block_at(block_pos.x as i32, block_pos.y as i32, block_pos.z as i32);
        let in_lava = match feet_block {
            Blocks::Lava { .. } | Blocks::FlowingLava { .. } => true,
            _ => false,
        };
        
        let was_in_lava = self.in_lava;
        self.in_lava = in_lava;
        
        // Only bounce when entering lava (not already in lava) and if enabled
        if in_lava && !was_in_lava && self.lava_bounce_enabled {
            let current_tick = world.tick_count;
            const LAVA_BOUNCE_COOLDOWN_TICKS: u64 = 10; // 500ms cooldown like Java
            
            // Use a shorter cooldown for 0 ping to prevent double bouncing
            const SHORT_BOUNCE_COOLDOWN_TICKS: u64 = 5; // 250ms for 0 ping scenarios
            
            if current_tick - self.lava_bounce_last_tick >= SHORT_BOUNCE_COOLDOWN_TICKS {
                
                // Apply upward velocity for lava bounce
                const LAVA_BOUNCE_VELOCITY: f64 = 0.5; // Upward velocity for instant surface bounce
                
                self.write_packet(&EntityVelocity {
                    entity_id: VarInt(self.entity_id),
                    velocity_x: (0.0 * 8000.0) as i16, // Keep horizontal motion
                    velocity_y: (LAVA_BOUNCE_VELOCITY * 8000.0) as i16, // Upward bounce
                    velocity_z: (0.0 * 8000.0) as i16, // Keep horizontal motion
                });
                
                self.lava_bounce_last_tick = current_tick;
                
            }
        }
    }
    
    pub fn collision_aabb(&self) -> AABB {
        let w = 0.3;
        let h = 1.8;
        AABB::new(
            DVec3::new(self.position.x - w, self.position.y, self.position.z - w),
            DVec3::new(self.position.x + w, self.position.y + h, self.position.z + w),
        )
    }

    /// Check for falling blocks collision when player moves
    fn check_fallingblocks_collision(&mut self) {
        let server = self.server_mut();
        let world = &mut server.world;
        let dungeon = &mut server.dungeon;
        
        // Find the room the player is in
        if let Some(room_index) = dungeon.get_room_at(self.position.x as i32, self.position.z as i32) {
            let room = dungeon.rooms.get_mut(room_index).unwrap();
            let player_pos = crate::server::block::block_position::BlockPos {
                x: self.position.x as i32,
                y: self.position.y as i32,
                z: self.position.z as i32,
            };
            
            // Check for falling blocks collision
            room.check_fallingblocks_collision(world, &player_pos);
        }
    }

    pub fn handle_left_click(&mut self) {
        
    }
    
    pub fn handle_right_click(&mut self) {
        if let Some(ItemSlot::Filled(item, _)) = self.inventory.get_hotbar_slot(self.held_slot as usize) {
            item.on_right_click(self).unwrap()
        }
    }
    
    /// Shoot Bonzo projectile with cooldown and delay like the Java version
    pub fn shoot_bonzo_projectile(&mut self) -> anyhow::Result<()> {
        use crate::server::items::bonzo_projectile::BonzoProjectileImpl;
        use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
        use crate::server::utils::sounds::Sounds;
        use crate::net::protocol::play::clientbound::SoundEffect;
        
        let current_tick = self.world_mut().tick_count;
        
        // Bonzo Staff cooldown (equivalent to bonzoCd in Java - 500ms = 10 ticks)
        const BONZO_COOLDOWN_TICKS: u64 = 3; // 150ms = 3 ticks at 20 TPS
        if current_tick - self.bonzo_last_shot_tick < BONZO_COOLDOWN_TICKS {
            return Ok(()); // Silently ignore if on cooldown
        }
        
        // Play ghast moan sound immediately (like Java version)
        self.write_packet(&SoundEffect {
            sound: Sounds::GhastMoan.id(),
            pos_x: self.position.x,
            pos_y: self.position.y,
            pos_z: self.position.z,
            volume: 1.0,
            pitch: 1.43,
        });
        
        // Calculate spawn position with forward offset
        let eye_height = 1.62; // Player eye height
        let yaw_rad = (self.yaw as f64).to_radians();
        let pitch_rad = (self.pitch as f64).to_radians();
        
        let direction = DVec3::new(
            -pitch_rad.cos() * yaw_rad.sin(),
            -pitch_rad.sin(),
            pitch_rad.cos() * yaw_rad.cos(),
        ).normalize();
        
        let spawn_offset = 0.15; // Small forward offset like Java version
        let spawn_pos = DVec3::new(
            self.position.x + direction.x * spawn_offset,
            self.position.y + eye_height + direction.y * spawn_offset,
            self.position.z + direction.z * spawn_offset,
        );
        
        // Spawn projectile immediately with full velocity (like Java version)
        let _projectile_id = self.world_mut().spawn_entity(
            spawn_pos,
            EntityMetadata::new(EntityVariant::BonzoProjectile),
            BonzoProjectileImpl::new(self.client_id, direction, 20.0), // Full speed immediately
        )?;
        
        // Update cooldown
        self.bonzo_last_shot_tick = current_tick;
        
        Ok(())
    }
    
    /// Shoot Jerry-Chine Gun projectile with cooldown and random spread like the Java version
    pub fn shoot_jerry_projectile(&mut self) -> anyhow::Result<()> {
        use crate::server::items::jerry_projectile::JerryProjectileImpl;
        use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
        
        let current_tick = self.world_mut().tick_count;
        
        // Jerry-Chine Gun cooldown (40ms = 0.8 ticks, round up to 1 tick)
        const JERRY_COOLDOWN_TICKS: u64 = 1;
        if current_tick - self.jerry_last_shot_tick < JERRY_COOLDOWN_TICKS {
            return Ok(()); // Silently ignore if on cooldown
        }
        
        // Calculate spawn position with forward offset and random spread
        let eye_height = 1.62; // Player eye height
        
        // Add random spread: yaw ±10 degrees, pitch ±10 degrees (like Java: rand.nextInt(21) - 10)
        let yaw_spread = rand::random::<f32>() * 21.0 - 10.0;
        let pitch_spread = rand::random::<f32>() * 21.0 - 10.0;
        
        let yaw_rad = ((self.yaw + yaw_spread) as f64).to_radians();
        let pitch_rad = ((self.pitch + pitch_spread) as f64).to_radians();
        
        let direction = DVec3::new(
            -pitch_rad.cos() * yaw_rad.sin(),
            -pitch_rad.sin(),
            pitch_rad.cos() * yaw_rad.cos(),
        ).normalize();
        
        let spawn_offset = 0.5; // Like Java version: 0.5D
        let spawn_pos = DVec3::new(
            self.position.x + direction.x * spawn_offset,
            self.position.y + eye_height + direction.y * spawn_offset,
            self.position.z + direction.z * spawn_offset,
        );
        
        // Spawn projectile immediately with full velocity (like Java version)
        let _projectile_id = self.world_mut().spawn_entity(
            spawn_pos,
            EntityMetadata::new(EntityVariant::JerryProjectile),
            JerryProjectileImpl::new(self.client_id, direction, 20.0), // Same speed as Bonzo
        )?;
        
        // Update cooldown
        self.jerry_last_shot_tick = current_tick;
        
        Ok(())
    }
    
    pub fn open_ui(&mut self, ui: UI) {
        self.current_ui = ui;
        // kind of temporary solution,
        // instead of just putting the item in an available slot if it is dragged
        if ui == UI::Inventory { 
            if let ItemSlot::Filled(item, _) = self.inventory.dragged_item { 
                self.write_packet(&SetSlot {
                    window_id: -1,
                    slot: 0,
                    item_stack: Some(item.get_item_stack()),
                })
            }
        }
        if let Some(container_data) = ui.get_container_data(self) {
            // team we forgot to check for wId exceeding 100
            if self.window_id > 99 {
                self.window_id = 1;
            } else {
                self.window_id += 1;
            }
            self.write_packet(&OpenWindow {
                window_id: self.window_id,
                inventory_type: "minecraft:chest".into(), // if this needs to be containers lmk
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
            self.write_packet(&WindowItems { // this breaks autoterms :fire:, in order to make it as like hypixel as possible we would need to use setslots
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
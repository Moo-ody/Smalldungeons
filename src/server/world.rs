use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::DestroyEntites;
use crate::net::var_int::VarInt;
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::chunk::chunk_grid::ChunkGrid;
use crate::server::entity::entity::{Entity, EntityId, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::entity::equipment::Equipment;
use crate::server::entity::spawn_equipped::{CombatState, AISuspended, AttackCooldown, CurrentTarget};
use crate::server::player::player::{ClientId, Player};
use crate::server::server::Server;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::player_list::PlayerList;
use crate::server::redstone::RedstoneSystem;
use crate::server::block::metadata::BlockMetadata;
// use crate::dungeon::p3::simon_says::SimonSays;
// use crate::dungeon::p3::terminal::TerminalManager;
// use crate::dungeon::p3::p3_manager::P3Manager;
use std::collections::HashMap;
use std::mem::take;
use uuid::Uuid;

pub mod tactical_insertion;
pub use tactical_insertion::{TacticalInsertionMarker, ScheduledSound, ScheduledFixedSound};

pub const VIEW_DISTANCE: u8 = 6;

pub struct World {
    /// Don't use directly!!, use .server_mut() instead
    /// This is unsafe,
    /// but since server should be alive for the entire program this is fine (I hope)
    pub server: *mut Server,

    pub chunk_grid: ChunkGrid,
    pub interactable_blocks: HashMap<BlockPos, BlockInteractAction>,

    pub player_info: PlayerList, // might need to be per player, not sure.

    // entity ids are always positive so they could theoretically be unsigned but minecraft uses signed ints in vanilla and casting might cause weird behavior, also assumes we ever reach the end of i32 though so it might be fine
    pub next_entity_id: i32,
    pub players: HashMap<ClientId, Player>,
    pub entities: HashMap<EntityId, (Entity, Box<dyn EntityImpl>)>,
    
    /// Equipment storage for entities
    pub entity_equipment: HashMap<EntityId, Equipment>,
    
    /// Combat state storage for entities (zombie arm pose control)
    pub entity_combat_state: HashMap<EntityId, CombatState>,
    
    /// AI suspension storage for entities
    pub entity_ai_suspended: HashMap<EntityId, AISuspended>,
    
    /// Attack cooldown storage for entities
    pub entity_attack_cooldown: HashMap<EntityId, AttackCooldown>,
    
    /// Current target storage for entities
    pub entity_current_target: HashMap<EntityId, CurrentTarget>,

    pub entities_for_removal: Vec<EntityId>,

    // pub commands: Vec<Command>
    
    // pub player_info: PlayerList,
    pub spawn_point: DVec3,
    pub spawn_yaw: f32,
    pub spawn_pitch: f32,
    
    // Scheduled tactical insertions (teleport back after delay)
    pub tactical_insertions: Vec<(TacticalInsertionMarker, Vec<ScheduledSound>)>,
    // Scheduled sounds at fixed positions
    pub scheduled_fixed_sounds: Vec<ScheduledFixedSound>,
    pub tick_count: u64,
    
    // Lava flow control
    pub stop_lava_flow: bool,
    
    // Redstone system for handling power transmission
    pub redstone_system: RedstoneSystem,
    
    // P3 Simon Says puzzle
    // pub simon_says: SimonSays,
    
    // P3 Terminal system
    // pub terminal_manager: TerminalManager,
    
    // P3 Manager
    // pub p3_manager: P3Manager,
}

impl World {

    pub fn new() -> World {
        World {
            server: std::ptr::null_mut(),

            chunk_grid: ChunkGrid::new(256, 128, 128),
            interactable_blocks: HashMap::new(),

            player_info: PlayerList::new(),

            next_entity_id: 1, // might have to start at 1
            players: HashMap::new(),
            entities: HashMap::new(),
            entity_equipment: HashMap::new(),
            entity_combat_state: HashMap::new(),
            entity_ai_suspended: HashMap::new(),
            entity_attack_cooldown: HashMap::new(),
            entity_current_target: HashMap::new(),
            entities_for_removal: Vec::new(),

            spawn_point: DVec3::ZERO,
            spawn_yaw: 0.0,
            spawn_pitch: 0.0,
            
            // NEW FIELDS
            tactical_insertions: Vec::new(),
            scheduled_fixed_sounds: Vec::new(),
            tick_count: 0,
            redstone_system: RedstoneSystem::new(),
            // simon_says: SimonSays::new(),
            // terminal_manager: TerminalManager::new(),
            // p3_manager: P3Manager::new(),
            
            stop_lava_flow: true, // Stop lava flow by default like Java version
        }
    }

    pub fn server_mut<'a>(&self) -> &'a mut Server {
        unsafe { self.server.as_mut().expect("server is null") }
    }

    pub fn new_entity_id(&mut self) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    pub fn spawn_entity<E : EntityImpl + 'static>(&mut self,position: DVec3, metadata: EntityMetadata, entity_impl: E,) -> anyhow::Result<EntityId> {
        self.spawn_entity_with_uuid(position, metadata, entity_impl, None)
    }

    pub fn spawn_entity_with_uuid<E : EntityImpl + 'static>(&mut self, position: DVec3, metadata: EntityMetadata, mut entity_impl: E, uuid: Option<Uuid>) -> anyhow::Result<EntityId> {
        let world_ptr: *mut World = self;
        let mut entity = Entity::new(
            world_ptr,
            self.new_entity_id(),
            position,
            metadata.clone(),
        );
        
        // Set UUID if provided
        entity.uuid = uuid;

        let chunk_x = (entity.position.x.floor() as i32) >> 4;
        let chunk_z = (entity.position.z.floor() as i32) >> 4;
        
        if let Some(chunk) = self.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
            chunk.insert_entity(entity.id);
            entity.write_spawn_packet(&mut chunk.packet_buffer);
            entity_impl.spawn(&mut entity, &mut chunk.packet_buffer);
        }

        let id = entity.id;
        self.entities.insert(id, (entity, Box::new(entity_impl)));
        Ok(id)
    }

    /// adds the entity id to 
    pub fn despawn_entity(&mut self, entity_id: EntityId) {
        self.entities_for_removal.push(entity_id)
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        // Increment tick counter
        self.tick_count = self.tick_count.wrapping_add(1);
        
        if !self.entities_for_removal.is_empty() {
            for entity_id in take(&mut self.entities_for_removal) {
                // Clean up equipment and combat state when entity is removed
                self.entity_equipment.remove(&entity_id);
                self.entity_combat_state.remove(&entity_id);
                self.entity_ai_suspended.remove(&entity_id);
                self.entity_attack_cooldown.remove(&entity_id);
                self.entity_current_target.remove(&entity_id);
                
                if let Some((mut entity, mut entity_impl)) = self.entities.remove(&entity_id) {
                    if let Some(chunk) = entity.chunk_mut() {
                        chunk.packet_buffer.write_packet(&DestroyEntites {
                            entities: vec![VarInt(entity.id)],
                        });
                        entity_impl.despawn(&mut entity, &mut chunk.packet_buffer);
                        chunk.remove_entity(&entity_id);
                    }
                }
            }
        }
        
        // Process AI suspension system (prevents immediate attack on spawn)
        self.process_ai_suspension_system();
        
        // Process combat state system (manages arm pose and attack cooldowns)
        self.process_combat_state_system();

        for (entity, entity_impl) in self.entities.values_mut() {
            let packet_buffer = if let Some(chunk) = entity.chunk_mut() {
                &mut chunk.packet_buffer
            } else {
                // throwaway packet buffer, doesn't feel like a good idea
                // however there is a chance that an entity might end up outside chunk grid 
                // and end up stuck if we tick only inside a valid chunk
                &mut PacketBuffer::new()
            };
            entity.tick(entity_impl, packet_buffer);
        }
        
                // Process scheduled tactical insertions
        tactical_insertion::process(self)?;
        
        // Process scheduled fixed-position sounds
        let now = self.tick_count;
        let mut remaining_sounds = Vec::new();
        for sound in self.scheduled_fixed_sounds.drain(..) {
            if sound.due_tick <= now {
                // Send sound to all players
                for (_, player) in &mut self.players {
                    player.write_packet(&crate::net::protocol::play::clientbound::SoundEffect {
                        sound: sound.sound.id(),
                        pos_x: sound.pos_x,
                        pos_y: sound.pos_y,
                        pos_z: sound.pos_z,
                        volume: sound.volume,
                        pitch: sound.pitch,
                    });
                }
            } else {
                remaining_sounds.push(sound);
            }
        }
        self.scheduled_fixed_sounds = remaining_sounds;
        
        // Process Simon Says puzzle timing system
        // self.simon_says.tick(self.tick_count);
        
        // Process Simon Says world tick actions
        // let actions_to_process = self.simon_says.get_pending_actions(self.tick_count);
        
        // for action in actions_to_process {
        //     match action {
        //         crate::dungeon::p3::simon_says::SolutionAction::RemoveButtons => {
        //             let button_positions = self.simon_says.get_button_positions();
        //             for pos in button_positions {
        //                 self.set_block_at(crate::server::block::blocks::Blocks::Air, pos.x, pos.y, pos.z);
        //             }
        //         }
        //         crate::dungeon::p3::simon_says::SolutionAction::ShowSeaLantern(pos) => {
        //             self.set_block_at(crate::server::block::blocks::Blocks::SeaLantern, pos.x, pos.y, pos.z);
        //         }
        //         crate::dungeon::p3::simon_says::SolutionAction::HideSeaLantern(pos) => {
        //             self.set_block_at(crate::server::block::blocks::Blocks::Obsidian, pos.x, pos.y, pos.z);
        //         }
        //         crate::dungeon::p3::simon_says::SolutionAction::ReplaceButtons => {
        //             let button_positions = self.simon_says.get_button_positions();
        //             for pos in button_positions {
        //                 self.set_block_at(
        //                     crate::server::block::blocks::Blocks::StoneButton { 
        //                         direction: crate::server::block::block_parameter::ButtonDirection::from_meta(2), 
        //                         powered: false 
        //                     }, 
        //                     pos.x, pos.y, pos.z
        //                 );
        //             }
        //         }
        //         crate::dungeon::p3::simon_says::SolutionAction::StartPuzzle => {
        //             self.simon_says.showing_solution = false;
        //         }
        //     }
        // }
        
        // Process redstone system updates
        // We'll handle redstone updates in the lever interaction instead
        
        
        Ok(())
    }
    
    pub fn set_block_at(&mut self, block: Blocks, x: i32, y: i32, z: i32) {
        self.chunk_grid.set_block_at(block, x, y, z);
    }
    
    pub fn get_block_at(&self, x: i32, y: i32, z: i32) -> Blocks {
        self.chunk_grid.get_block_at(x, y, z)
    }
    
    pub fn set_spawn_point(&mut self, position: DVec3, yaw: f32, pitch: f32) {
        self.spawn_point = position;
        self.spawn_yaw = yaw;
        self.spawn_pitch = pitch;
    }
    
    pub fn fill_blocks(&mut self, block: Blocks, start: BlockPos, end: BlockPos) {
        iterate_blocks(start, end, |x, y, z| {
            self.set_block_at(block, x, y, z)
        })
    }

    pub fn get_next_entity_id(&mut self) -> u32 {
        let id = self.next_entity_id as u32;
        self.next_entity_id += 1;
        id
    }

    pub fn destroy_entity(&mut self, entity_id: u32) {
        self.entities_for_removal.push(entity_id as i32);
    }

    /// Ensure the chunk at the given position is loaded (exists)
    pub fn ensure_chunk_loaded(&mut self, pos: DVec3) {
        let chunk_x = (pos.x.floor() as i32) >> 4;
        let chunk_z = (pos.z.floor() as i32) >> 4;
        
        // Chunk will be created if it doesn't exist when we try to access it
        if self.chunk_grid.get_chunk(chunk_x, chunk_z).is_none() {
            // Chunk doesn't exist yet, but it will be created when needed
            // The chunk_grid will handle this automatically
        }
    }

    /// Get equipment for an entity
    pub fn get_equipment(&self, entity_id: EntityId) -> Option<&Equipment> {
        self.entity_equipment.get(&entity_id)
    }
    
    /// Get combat state for an entity
    pub fn get_combat_state(&self, entity_id: EntityId) -> Option<&CombatState> {
        self.entity_combat_state.get(&entity_id)
    }
    
    /// Get mutable combat state for an entity
    pub fn get_combat_state_mut(&mut self, entity_id: EntityId) -> Option<&mut CombatState> {
        self.entity_combat_state.get_mut(&entity_id)
    }
    
    /// Set combat state for an entity
    pub fn set_combat_state(&mut self, entity_id: EntityId, state: CombatState) {
        self.entity_combat_state.insert(entity_id, state);
    }
    
    /// Get AI suspension for an entity
    pub fn get_ai_suspended(&self, entity_id: EntityId) -> Option<&AISuspended> {
        self.entity_ai_suspended.get(&entity_id)
    }
    
    /// Set AI suspension for an entity
    pub fn set_ai_suspended(&mut self, entity_id: EntityId, suspended: AISuspended) {
        self.entity_ai_suspended.insert(entity_id, suspended);
    }
    
    /// Get attack cooldown for an entity
    pub fn get_attack_cooldown(&self, entity_id: EntityId) -> Option<&AttackCooldown> {
        self.entity_attack_cooldown.get(&entity_id)
    }
    
    /// Get mutable attack cooldown for an entity
    pub fn get_attack_cooldown_mut(&mut self, entity_id: EntityId) -> Option<&mut AttackCooldown> {
        self.entity_attack_cooldown.get_mut(&entity_id)
    }
    
    /// Set attack cooldown for an entity
    pub fn set_attack_cooldown(&mut self, entity_id: EntityId, cooldown: AttackCooldown) {
        self.entity_attack_cooldown.insert(entity_id, cooldown);
    }
    
    /// Send metadata update for an entity to all players
    pub fn send_metadata_update(&mut self, entity_id: EntityId) {
        if let Some((entity, _)) = self.entities.get(&entity_id) {
            for player in self.players.values_mut() {
                player.write_packet(&crate::net::protocol::play::clientbound::PacketEntityMetadata {
                    entity_id: crate::net::var_int::VarInt(entity_id),
                    metadata: entity.metadata.clone(),
                });
            }
        }
    }
    
    /// Process AI suspension system - prevents immediate attack on spawn
    fn process_ai_suspension_system(&mut self) {
        let mut to_remove = Vec::new();
        let mut entities_to_update = Vec::new();
        
        for (entity_id, ai_suspended) in &mut self.entity_ai_suspended {
            if ai_suspended.ticks_left > 0 {
                ai_suspended.ticks_left -= 1;
                // Keep AI disabled during suspension to maintain idle pose
                entities_to_update.push(*entity_id);
            }
            if ai_suspended.ticks_left == 0 {
                to_remove.push(*entity_id);
            }
        }
        
        // Remove expired suspensions but keep AI disabled for zombies
        for entity_id in to_remove {
            self.entity_ai_suspended.remove(&entity_id);
            entities_to_update.push(entity_id);
        }
        
        // Ensure AI remains disabled for all zombie entities to maintain idle pose
        for entity_id in entities_to_update {
            if let Some((entity, _)) = self.entities.get_mut(&entity_id) {
                if let EntityVariant::Zombie { .. } = entity.metadata.variant {
                    entity.metadata.ai_disabled = true; // Keep arms down
                }
            }
        }
    }
    
    /// Process combat state system - manages arm pose and attack cooldowns
    fn process_combat_state_system(&mut self) {
        // Process attack cooldowns
        for (_, cooldown) in &mut self.entity_attack_cooldown {
            if cooldown.ticks > 0 {
                cooldown.ticks -= 1;
            }
        }
        
        // Process combat states and swing animations
        let mut entities_to_update = Vec::new();
        for (entity_id, combat_state) in &mut self.entity_combat_state {
            // Handle swing animation timing
            if combat_state.aggressive && combat_state.swing_ticks > 0 {
                combat_state.swing_ticks -= 1;
                if combat_state.swing_ticks == 0 {
                    // End of swing - return to idle pose
                    combat_state.aggressive = false;
                    entities_to_update.push(*entity_id);
                }
            }
        }
        
        // Update metadata for entities that changed state
        for entity_id in entities_to_update {
            if let Some((entity, _)) = self.entities.get_mut(&entity_id) {
                if let EntityVariant::Zombie { is_child, is_villager, is_converting, .. } = entity.metadata.variant {
                    entity.metadata.variant = EntityVariant::Zombie {
                        is_child,
                        is_villager,
                        is_converting,
                        is_attacking: false, // Return to idle pose
                    };
                }
                // Send metadata update to all players
                self.send_metadata_update(entity_id);
            }
        }
    }
}

/// iterates over the blocks in area between start and end
/// and runs a function
#[inline(always)]
pub fn iterate_blocks<F>(
    start: BlockPos,
    end: BlockPos,
    mut callback: F,
) where 
    F : FnMut(i32, i32, i32)
{
    let x0 = start.x.min(end.x);
    let y0 = start.y.min(end.y);
    let z0 = start.z.min(end.z);

    let x1 = start.x.max(end.x);
    let y1 = start.y.max(end.y);
    let z1 = start.z.max(end.z);

    for x in x0..=x1 {
        for z in z0..=z1 {
            for y in y0..=y1 {
                callback(x, y, z);
            }
        }
    }
}
use anyhow;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{EntityAttach, EntityEquipment, EntityTeleport};
use crate::net::var_int::VarInt;
use crate::server::entity::entity::{Entity, EntityId, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::entity::equipment::Equipment;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;

/// Combat state component for zombie entities
/// Controls arm pose via metadata - false = idle (arms down), true = attack (arms raised)
#[derive(Clone, Copy, Debug, Default)]
pub struct CombatState {
    pub aggressive: bool,
    pub swing_ticks: u16,    // ticks left in current swing animation
}

/// AI suspension component to prevent immediate attack on spawn
#[derive(Clone, Copy, Debug)]
pub struct AISuspended { 
    pub ticks_left: u8 
}

/// Attack cooldown component
#[derive(Clone, Copy, Debug)]
pub struct AttackCooldown { 
    pub ticks: u16 
}

impl AttackCooldown { 
    pub fn ready() -> Self { 
        Self { ticks: 0 } 
    } 
}

/// Current target component (optional, for future use)
#[derive(Clone, Copy, Debug)]
pub struct CurrentTarget(pub Option<EntityId>);

/// Spawn options for equipped zombies
#[derive(Clone, Copy)]
pub struct SpawnOpts {
    pub pos:  DVec3,    // absolute position
    pub yaw:  f32,
    pub pitch: f32,
    pub hp:   Option<f32>,
    pub tags: &'static [&'static str], // e.g., ["starred","commander"]
}

/// Spawns a zombie that is already fully equipped on first frame
pub fn spawn_equipped_zombie(
    world: &mut World,
    mut eq: Equipment,
    opts: SpawnOpts,
) -> EntityId {
    // 1) Warm the chunk BEFORE registration/broadcast
    world.ensure_chunk_loaded(opts.pos);

    // 2) Prepare items according to flags once up front
    if eq.unbreakable {
        eq.apply_unbreakable();
    }

    // 3) Create zombie entity metadata with arms down (idle pose)
    let mut metadata = EntityMetadata::new(EntityVariant::Zombie {
        is_child: false,
        is_villager: false,
        is_converting: false,
        is_attacking: false, // Arms down - idle pose
    });
    metadata.ai_disabled = true; // Disable AI to keep arms down (idle pose)

    // 4) Build entity and register it first
    let world_ptr: *mut World = world as *mut World;
    let entity_id = world.new_entity_id();
    let mut entity = Entity::new(world_ptr, entity_id, opts.pos, metadata.clone());
    entity.yaw = opts.yaw;
    entity.pitch = opts.pitch;

    // 5) Add combat state components BEFORE registration (critical for spawn frame)
    world.set_combat_state(entity_id, CombatState { 
        aggressive: false, 
        swing_ticks: 0 
    }); // First snapshot = idle (arms down)
    
    world.set_ai_suspended(entity_id, AISuspended { ticks_left: 10 }); // Block AI for 10 ticks to ensure arms stay down
    world.set_attack_cooldown(entity_id, AttackCooldown::ready()); // Not mid-swing

    // 6) Store equipment before broadcasting
    world.entity_equipment.insert(entity_id, eq.clone());

    // 7) Insert entity into chunk and world - use proper chunk system for resyncing
    let chunk_x = (entity.position.x.floor() as i32) >> 4;
    let chunk_z = (entity.position.z.floor() as i32) >> 4;
    
    // Create a zombie entity implementation with combat state management
    let mut entity_impl = Box::new(ZombieImpl { entity_id });

    // Ensure initial metadata reflects combat state (idle pose)
    if let EntityVariant::Zombie { is_child, is_villager, is_converting, .. } = entity.metadata.variant {
        entity.metadata.variant = EntityVariant::Zombie {
            is_child,
            is_villager,
            is_converting,
            is_attacking: false, // Start with idle pose (arms down)
        };
    }

    // Insert entity into chunk first
    if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
        chunk.insert_entity(entity.id);
        entity.write_spawn_packet(&mut chunk.packet_buffer);
        entity_impl.spawn(&mut entity, &mut chunk.packet_buffer);
        
        // Also add equipment packets to chunk buffer for resyncing
        send_equipment_packets(&mut chunk.packet_buffer, entity_id, &eq);
    }

    // 7) Send spawn and equipment to current players before inserting into world
    for player in world.players.values_mut() {
        // Send spawn packet with metadata (arms down)
        entity.write_spawn_packet(&mut player.packet_buffer);
        
        // Send explicit metadata update to ensure arms are down
        player.write_packet(&crate::net::protocol::play::clientbound::PacketEntityMetadata {
            entity_id: crate::net::var_int::VarInt(entity_id),
            metadata: entity.metadata.clone(),
        });
        
        // Send equipment packets
        send_equipment_to_player(entity_id, &eq, player);
        
        // Send another metadata update after equipment to ensure arms stay down
        player.write_packet(&crate::net::protocol::play::clientbound::PacketEntityMetadata {
            entity_id: crate::net::var_int::VarInt(entity_id),
            metadata: entity.metadata.clone(),
        });
        
        player.flush_packets();
    }

    // 8) Insert entity into world map after sending packets
    world.entities.insert(entity_id, (entity, entity_impl));

    entity_id
}

/// Send equipment packets for all equipped slots in the correct order
pub fn send_equipment_packets(buffer: &mut PacketBuffer, entity_id: EntityId, eq: &Equipment) {
    // Equipment slot order: MAIN_HAND (0), BOOTS (1), LEGS (2), CHEST (3), HELMET (4)
    
    if let Some(ref item) = eq.main_hand {
        buffer.write_packet(&EntityEquipment {
            entity_id: VarInt(entity_id),
            item_slot: 0, // main hand
            item_stack: Some(item.clone()),
        });
    }

    if let Some(ref item) = eq.boots {
        buffer.write_packet(&EntityEquipment {
            entity_id: VarInt(entity_id),
            item_slot: 1, // boots
            item_stack: Some(item.clone()),
        });
    }

    if let Some(ref item) = eq.legs {
        buffer.write_packet(&EntityEquipment {
            entity_id: VarInt(entity_id),
            item_slot: 2, // legs
            item_stack: Some(item.clone()),
        });
    }

    if let Some(ref item) = eq.chest {
        buffer.write_packet(&EntityEquipment {
            entity_id: VarInt(entity_id),
            item_slot: 3, // chest
            item_stack: Some(item.clone()),
        });
    }

    if let Some(ref item) = eq.helmet {
        buffer.write_packet(&EntityEquipment {
            entity_id: VarInt(entity_id),
            item_slot: 4, // helmet
            item_stack: Some(item.clone()),
        });
    }
}

/// Helper function to broadcast equipment to a specific player (for resync)
pub fn send_equipment_to_player(
    entity_id: EntityId,
    eq: &Equipment,
    player: &mut crate::server::player::player::Player,
) {
    send_equipment_packets(&mut player.packet_buffer, entity_id, eq);
}


/// Following nametag implementation for armor stands
struct FollowingNametagImpl {
    host_entity_id: EntityId,
    y_offset: f64,
}

impl FollowingNametagImpl {
    fn new(host_entity_id: EntityId, y_offset: f64) -> Self {
        Self { host_entity_id, y_offset }
    }
}

impl EntityImpl for FollowingNametagImpl {
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Re-attach the nametag to its host entity when respawning (e.g., on chunk refresh)
        packet_buffer.write_packet(&EntityAttach {
            entity_id: entity.id,
            vehicle_id: self.host_entity_id,
            leash: false,
        });
    }

    fn tick(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Get the host entity position and update our position accordingly
        let world = entity.world_mut();
        if let Some((host_entity, _)) = world.entities.get(&self.host_entity_id) {
            let target_pos = DVec3::new(
                host_entity.position.x,
                host_entity.position.y + self.y_offset,
                host_entity.position.z
            );

            // Only teleport if position changed significantly (to avoid spam)
            let distance = entity.position.distance_to(&target_pos);
            if distance > 0.01 {
                entity.position = target_pos;
                
                // Send teleport packet to update position
                packet_buffer.write_packet(&EntityTeleport {
                    entity_id: entity.id,
                    pos_x: target_pos.x,
                    pos_y: target_pos.y,
                    pos_z: target_pos.z,
                    yaw: entity.yaw,
                    pitch: entity.pitch,
                    on_ground: false,
                });
            }
        }
    }
}

/// Spawns an invisible armor stand nametag that follows the given entity
pub fn spawn_following_nametag(
    world: &mut World,
    host_entity_id: EntityId,
    nametag_text: &str,
    y_offset: f64,
) -> anyhow::Result<EntityId> {
    // Get host entity position
    let host_pos = if let Some((host_entity, _)) = world.entities.get(&host_entity_id) {
        host_entity.position
    } else {
        return Err(anyhow::anyhow!("Host entity not found"));
    };

    let nametag_pos = DVec3::new(
        host_pos.x,
        host_pos.y + y_offset,
        host_pos.z
    );

    // Create invisible armor stand metadata with custom name
    let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
    metadata.is_invisible = true;
    metadata.custom_name = Some(nametag_text.to_string());
    metadata.custom_name_visible = true;
    metadata.ai_disabled = true;

    // Spawn the armor stand
    let nametag_id = world.new_entity_id();
    let world_ptr: *mut World = world as *mut World;
    let mut nametag_entity = Entity::new(world_ptr, nametag_id, nametag_pos, metadata.clone());

    // Create the following nametag implementation first
    let mut nametag_impl = Box::new(FollowingNametagImpl::new(host_entity_id, y_offset));

    // Insert into chunk and use proper chunk system for resyncing
    let chunk_x = (nametag_pos.x.floor() as i32) >> 4;
    let chunk_z = (nametag_pos.z.floor() as i32) >> 4;
    
    if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
        chunk.insert_entity(nametag_entity.id);
        nametag_entity.write_spawn_packet(&mut chunk.packet_buffer);
        nametag_impl.spawn(&mut nametag_entity, &mut chunk.packet_buffer);
    }

    // Register the entity in the world
    world.entities.insert(nametag_id, (nametag_entity, nametag_impl));

    Ok(nametag_id)
}

/// Zombie entity implementation with combat state management
struct ZombieImpl {
    entity_id: EntityId,
}

impl EntityImpl for ZombieImpl {
    fn tick(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        let world = entity.world_mut();
        
        // AI suspension and combat state are handled by world systems
        // Just ensure AI stays disabled to maintain idle pose (arms down)
        entity.metadata.ai_disabled = true;
        
        // Handle combat state and arm pose
        if let Some(combat_state) = world.get_combat_state_mut(self.entity_id) {
            // Handle swing animation timing
            if combat_state.aggressive && combat_state.swing_ticks > 0 {
                combat_state.swing_ticks -= 1;
                if combat_state.swing_ticks == 0 {
                    // End of swing - return to idle pose
                    combat_state.aggressive = false;
                    self.update_metadata_aggressive_state(entity, world, false);
                }
            }
            
            // Handle attack cooldown
            if let Some(cooldown) = world.get_attack_cooldown_mut(self.entity_id) {
                if cooldown.ticks > 0 {
                    cooldown.ticks -= 1;
                }
            }
        }
    }
    
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Ensure metadata reflects current combat state on spawn
        let world = entity.world_mut();
        if let Some(combat_state) = world.get_combat_state(self.entity_id) {
            self.update_metadata_aggressive_state(entity, world, combat_state.aggressive);
        }
    }
}

impl ZombieImpl {
    fn update_metadata_aggressive_state(&self, entity: &mut Entity, world: &mut World, aggressive: bool) {
        // Update the zombie metadata to reflect aggressive state
        if let EntityVariant::Zombie { is_child, is_villager, is_converting, .. } = &entity.metadata.variant {
            // Update the is_attacking field based on combat state
            let new_variant = EntityVariant::Zombie {
                is_child: *is_child,
                is_villager: *is_villager,
                is_converting: *is_converting,
                is_attacking: aggressive,
            };
            entity.metadata.variant = new_variant;
        }
        
        // Send metadata update to all players (done after the entity update to avoid borrow conflicts)
        world.send_metadata_update(self.entity_id);
    }
}

/// Simple entity implementation for other entities (fallback)
struct NoEntityImpl;

impl EntityImpl for NoEntityImpl {
    fn tick(&mut self, _: &mut Entity, _: &mut PacketBuffer) {
        // Basic entity behavior can be added here later
    }
}

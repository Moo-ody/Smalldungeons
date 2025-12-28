use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityImpl, EntityId, NoEntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::utils::aabb::AABB;
use crate::server::utils::direction::Direction;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::nbt::nbt::{NBT, NBTNode};
use crate::server::utils::nbt::serialize::TAG_COMPOUND_ID;
use std::collections::HashMap;
use crate::server::world::World;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{CollectItem, EntityEquipment, EntityTeleport, EntityVelocity, Particles, SoundEffect};
use crate::net::var_int::VarInt;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum SecretType {
    WitherEssence {
        // have rotation here
    },
    Chest {
        direction: Direction
    },
    Item {
        item: ItemStack
    },
    // ill do this one later
    Bat,
    // New secret types from secrets.json
    RegularChest {
        direction: Direction
    },
    RegularEssence,
    BatSpawn {
        entity_id: Option<u32> // Track spawned bat entity
    },
    BatDie,
    ItemSpawn, // Item is created fresh at spawn time
    SecretChest {
        direction: Direction
    },
    SecretEssence,
}

#[derive(Debug)]
pub struct DungeonSecret {
    pub secret_type: SecretType,
    pub block_pos: BlockPos, // might not even need?
    pub spawn_aabb: AABB,
    pub has_spawned: bool,
    pub obtained: bool,
    pub counted: bool, // Track if this secret has been counted in room's found_secrets
    pub bat_entity_id: Option<i32>, // Track bat entity for batsp/batdie
    pub bat_spawn_tick: Option<u64>, // Track when bat was spawned for batdie
}

// when this is integrated into rooms, remove this and just inline the spawning stuff
pub fn tick(dungeon_secret: &Rc<RefCell<DungeonSecret>>, player: &Player) {
    let mut secret = dungeon_secret.borrow_mut();
    if !secret.has_spawned && player.collision_aabb().intersects(&secret.spawn_aabb) {
        secret.has_spawned = true;
        DungeonSecret::spawn_into_world(dungeon_secret, secret, player.world_mut())
    }
}

impl DungeonSecret {
    /// Create the redstone key skull NBT data
    /// For tile entities in 1.8, use "Owner" not "SkullOwner"
    pub fn create_redstone_key_skull_nbt() -> (String, NBTNode) {
        // Create texture compound with Value and Signature
        let texture_compound = NBTNode::Compound({
            let mut map = HashMap::new();
            map.insert("Value".into(), NBTNode::String("eyJ0aW1lc3RhbXAiOjE1NTk1ODAzNjI1NTMsInByb2ZpbGVJZCI6ImU3NmYwZDlhZjc4MjQyYzM5NDY2ZDY3MjE3MzBmNDUzIiwicHJvZmlsZU5hbWUiOiJLbGxscmFoIiwic2lnbmF0dXJlUmVxdWlyZWQiOnRydWUsInRleHR1cmVzIjp7IlNLSU4iOnsidXJsIjoiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS8yZjI0ZWQ2ODc1MzA0ZmE0YTFmMGM3ODViMmNiNmE2YTcyNTYzZTlmM2UyNGVhNTVlMTgxNzg0NTIxMTlhYTY2In19fQ==".to_string()));
            map.insert("Signature".into(), NBTNode::String("dYEJC8GTGdDESqHrQn22ShF5sJWO7u3jpG2hKPSD9Yords2BsESC3RdrImpeMMyD9oS4INbtsDPAPOoev9wrQS3JTkJWjHdgrwd33UqL9IHvQOmqKLAX5gLIeNNzJ3djG23oVsQ6JuW/OfnhbwpSxFNNzfwwtOjzDaiS4LLCqvkdQziUCTdfuBbvSaI6Jae0HBk2qXIHJ9Kjr9sSmcFhaDXLXj/lhkdxXCCGD+5XAxhR47ORnBT2qhHlZdK3bvNts41Kk6qC6Gzz7JdpGZPhnGCkK4FZzr/MDYObQuWmCOJQtI4QavjKOqO97AcY8IKyyMgkAJHRyqlO+8Y1sCwA7Fl5vB1lF9gUscVKLNRrT46Skg9lPWjIl3xAfEHdWt0HyU4GJb6tBPP/b2qm5vOAQ9JnaJuMiJm3ISfocz+NxlpmUli/vrsG79wwB4hT1wJAUfLwoi+Z0y0oG+FP45yAnIO3ORA/WjiqfnWu76kPaSenUEMV81IQtAJB835fzV7VLZWR5EkN/knuMWPuAGz0kdG1Raevi7yJC3wkmIRs1B523IB7Reuq14IgFXGw1J1i4Df00ULTkdWgMtPSWOlKGoT7iEBmYtnvPKQ3ZkzkoG9HZOq+JN5UpacfipmR5kI16vKYjRGKThDokifn6PL9Lfo0DYtJb+96/sp2gf6VD4o=".to_string()));
            map
        });
        
        // Create Owner compound with Id, hypixelPopulated, and Properties
        // Use "Owner" not "SkullOwner" for tile entities in 1.8
        NBT::compound("Owner", vec![
            NBT::string("Id", "2134ab1c-7c78-30e1-8513-a6346c2344fd"),
            NBT::byte("hypixelPopulated", 1),
            NBT::compound("Properties", vec![
                NBT::list("textures", TAG_COMPOUND_ID, vec![texture_compound])
            ])
        ])
    }

    /// Create the wither skull NBT data
    /// For tile entities in 1.8, use "Owner" not "SkullOwner"
    pub fn create_wither_skull_nbt() -> (String, NBTNode) {
        // Create texture compound with Value and Signature
        let texture_compound = NBTNode::Compound({
            let mut map = HashMap::new();
            map.insert("Value".into(), NBTNode::String("ewogICJ0aW1lc3RhbXAiIDogMTYwMzYxMDQ0MzU4MywKICAicHJvZmlsZUlkIiA6ICIzM2ViZDMyYmIzMzk0YWQ5YWM2NzBjOTZjNTQ5YmE3ZSIsCiAgInByb2ZpbGVOYW1lIiA6ICJEYW5ub0JhbmFubm9YRCIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9lNDllYzdkODJiMTQxNWFjYWUyMDU5Zjc4Y2QxZDE3NTRiOWRlOWIxOGNhNTlmNjA5MDI0YzRhZjg0M2Q0ZDI0IgogICAgfQogIH0KfQ==".to_string()));
            map.insert("Signature".into(), NBTNode::String("Mnf7PDLe+FPiO+wQ2St6XNRiiIXtZ3GuPTcLlM7pNQ6d6MXuzI7xXG24qaAMFuVwMB+F3dLYcaFlc+bWyi3Qm9msSq2mMUXdvzTamAslZHcdcTFNpppkYgdvkOhWK7W/amQyd2Q+pLDECe8Mg6gxBY17+xfaWlIynzEWEmHR+ye+hTC44kgiTZaYiRg7gpU002deY8WpX875cc5zJIroxVR52qHIV+suIMPwq47mpCp520J9R1HuYvvP/V3+PwL7skMlC1F/HHkG5A13fvSKMqq9XMsdqXR8qvWlcL5IQTS7ijtD9TZo8jcmhz/7HCXuJ912I1GqJp4hZ0Lqa0NB0TuI/giHr2i4yNzORe6oan47bpMXLoZWIrYZIOsF6wSObhwniF1jM/zUEkum9XswRImIvYYlmyLH+Kkh5uQJm244rOLPXmOZEid6PW5bhaSRpMOMpxboeOtjLbGC56Ev+DwoI37SrAYY6/LC7HwjVhvkcsLd/9BrF+Wl10bdLdsJEbd+TII59/45MM1x7+xgeAFU/ip0TjkMPfRLdNmfxOGssMFZOaM55iOb+8t4tOvXxnqeXpFCByDgPnqKV5zPXS1XMF2+5qEAv7ZKrqK8BLAHbWsKHHOMt1hJ8K+EgYfRDKq72YvN01ST288ysUv8b5stRu8O5uC+KvZXtnlGrKc=".to_string()));
            map
        });
        
        // Create Owner compound with Id and Properties
        // Use "Owner" not "SkullOwner" for tile entities in 1.8
        NBT::compound("Owner", vec![
            NBT::string("Id", "e0f3e929-869e-3dca-9504-54c666ee6f23"),
            NBT::compound("Properties", vec![
                NBT::list("textures", TAG_COMPOUND_ID, vec![texture_compound])
            ])
        ])
    }

    pub fn new(secret_type: SecretType, position: BlockPos, spawn_radius: f64) -> Self {
        Self {
            secret_type,
            block_pos: position,
            spawn_aabb: {
                let (x, y, z) = (position.x as f64, position.y as f64, position.z as f64);
                // 8-block bounding box in all directions (8 blocks out from origin on each side)
                // This creates a 16x16x16 box centered on the secret position
                AABB {
                    min: DVec3::new(x - spawn_radius, y - spawn_radius, z - spawn_radius),
                    max: DVec3::new(x + spawn_radius, y + spawn_radius, z + spawn_radius),
                }
            },
            has_spawned: false,
            obtained: false,
            counted: false,
            bat_entity_id: None,
            bat_spawn_tick: None,
        }
    }
    
    pub fn spawn_into_world(
        secret_rc: &Rc<RefCell<DungeonSecret>>,
        mut secret: RefMut<DungeonSecret>,
        world: &mut World
    ) {
        match &secret.secret_type {
            SecretType::WitherEssence { .. } => {
                // Set skull block in world (this sends BlockChange automatically)
                world.set_block_at(
                    Blocks::Skull { direction: Direction::Up, no_drop: false },
                    secret.block_pos.x,
                    secret.block_pos.y,
                    secret.block_pos.z
                );
                // Send UpdateBlockEntity with full tile entity NBT
                // SkullType 3 = player head (required in 1.8)
                use crate::net::protocol::play::clientbound::UpdateBlockEntity;
                use crate::server::utils::nbt::serialize::serialize_nbt;
                let skull_owner = Self::create_wither_skull_nbt();
                let full_te_nbt = NBT::with_nodes(vec![
                    NBT::string("id", "Skull"),
                    NBT::int("x", secret.block_pos.x),
                    NBT::int("y", secret.block_pos.y),
                    NBT::int("z", secret.block_pos.z),
                    NBT::byte("SkullType", 3), // 3 = player head
                    skull_owner,
                ]);
                let nbt_bytes = serialize_nbt(&full_te_nbt);
                let update_packet = UpdateBlockEntity {
                    block_pos: secret.block_pos,
                    action: 4, // 4 = skull update in 1.8
                    nbt_data: Some(nbt_bytes.clone()),
                };
                for (_, player) in &mut world.players {
                    player.write_packet(&update_packet);
                }
                let chunk_x = secret.block_pos.x >> 4;
                let chunk_z = secret.block_pos.z >> 4;
                if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                    chunk.packet_buffer.write_packet(&update_packet);
                }
                world.interactable_blocks.insert(secret.block_pos, BlockInteractAction::WitherEssence {
                    secret: secret_rc.clone()
                });
            }
            SecretType::Chest { direction } => {
                world.set_block_at(
                    Blocks::Chest { direction: *direction },
                    secret.block_pos.x,
                    secret.block_pos.y,
                    secret.block_pos.z,
                );
                world.interactable_blocks.insert(secret.block_pos, BlockInteractAction::Chest {
                    secret: secret_rc.clone()
                });
            }
            SecretType::Item { item } => {
                world.spawn_entity(
                    DVec3::from_centered(&secret.block_pos),
                    EntityMetadata::new(EntityVariant::DroppedItem {
                        item: item.clone()
                    }),
                    NoEntityImpl,
                ).unwrap();
            }
            SecretType::Bat => {}
            SecretType::RegularChest { direction } => {
                world.set_block_at(
                    Blocks::Chest { direction: *direction },
                    secret.block_pos.x,
                    secret.block_pos.y,
                    secret.block_pos.z,
                );
                world.interactable_blocks.insert(secret.block_pos, BlockInteractAction::Chest {
                    secret: secret_rc.clone()
                });
            }
            SecretType::RegularEssence => {
                world.set_block_at(
                    Blocks::Skull { direction: Direction::Up, no_drop: false },
                    secret.block_pos.x,
                    secret.block_pos.y,
                    secret.block_pos.z
                );
                use crate::net::protocol::play::clientbound::UpdateBlockEntity;
                use crate::server::utils::nbt::serialize::serialize_nbt;
                let skull_owner = Self::create_wither_skull_nbt();
                let full_te_nbt = NBT::with_nodes(vec![
                    NBT::string("id", "Skull"),
                    NBT::int("x", secret.block_pos.x),
                    NBT::int("y", secret.block_pos.y),
                    NBT::int("z", secret.block_pos.z),
                    NBT::byte("SkullType", 3), // 3 = player head
                    skull_owner,
                ]);
                let nbt_bytes = serialize_nbt(&full_te_nbt);
                let update_packet = UpdateBlockEntity {
                    block_pos: secret.block_pos,
                    action: 4, // 4 = skull update in 1.8
                    nbt_data: Some(nbt_bytes.clone()),
                };
                for (_, player) in &mut world.players {
                    player.write_packet(&update_packet);
                }
                let chunk_x = secret.block_pos.x >> 4;
                let chunk_z = secret.block_pos.z >> 4;
                if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                    chunk.packet_buffer.write_packet(&update_packet);
                }
                world.interactable_blocks.insert(secret.block_pos, BlockInteractAction::WitherEssence {
                    secret: secret_rc.clone()
                });
            }
            SecretType::BatSpawn { .. } => {
                // Spawn bat entity and track it
                let bat_pos = DVec3::new(
                    secret.block_pos.x as f64 + 0.5,
                    secret.block_pos.y as f64 + 0.5,
                    secret.block_pos.z as f64 + 0.5
                );
                if let Ok(entity_id) = world.spawn_entity(
                    bat_pos,
                    EntityMetadata::new(EntityVariant::Bat { hanging: false }),
                    NoEntityImpl,
                ) {
                    secret.bat_entity_id = Some(entity_id);
                }
            }
            SecretType::BatDie => {
                // Spawn bat and schedule death after 0.25s (5 ticks)
                let bat_pos = DVec3::new(
                    secret.block_pos.x as f64 + 0.5,
                    secret.block_pos.y as f64 + 0.5,
                    secret.block_pos.z as f64 + 0.5
                );
                if let Ok(entity_id) = world.spawn_entity(
                    bat_pos,
                    EntityMetadata::new(EntityVariant::Bat { hanging: false }),
                    NoEntityImpl,
                ) {
                    secret.bat_entity_id = Some(entity_id);
                    // bat_spawn_tick will be set by the dungeon tick system
                }
            }
            SecretType::ItemSpawn => {
                // Create item fresh at spawn time
                use crate::dungeon::room::secrets_loader::create_spirit_leap_item;
                let item = create_spirit_leap_item();
                
                // Spawn with SecretItemEntityImpl for pickup detection
                if let Ok(_entity_id) = world.spawn_entity(
                    DVec3::from_centered(&secret.block_pos),
                    EntityMetadata::new(EntityVariant::DroppedItem {
                        item: item.clone()
                    }),
                    SecretItemEntityImpl {
                        secret: secret_rc.clone(),
                    },
                ) {
                    // Entity spawned successfully
                }
            }
            SecretType::SecretChest { direction } => {
                world.set_block_at(
                    Blocks::Chest { direction: *direction },
                    secret.block_pos.x,
                    secret.block_pos.y,
                    secret.block_pos.z,
                );
                world.interactable_blocks.insert(secret.block_pos, BlockInteractAction::Chest {
                    secret: secret_rc.clone()
                });
            }
            SecretType::SecretEssence => {
                world.set_block_at(
                    Blocks::Skull { direction: Direction::Up, no_drop: false },
                    secret.block_pos.x,
                    secret.block_pos.y,
                    secret.block_pos.z
                );
                use crate::net::protocol::play::clientbound::UpdateBlockEntity;
                use crate::server::utils::nbt::serialize::serialize_nbt;
                let skull_owner = Self::create_wither_skull_nbt();
                let full_te_nbt = NBT::with_nodes(vec![
                    NBT::string("id", "Skull"),
                    NBT::int("x", secret.block_pos.x),
                    NBT::int("y", secret.block_pos.y),
                    NBT::int("z", secret.block_pos.z),
                    NBT::byte("SkullType", 3), // 3 = player head
                    skull_owner,
                ]);
                let nbt_bytes = serialize_nbt(&full_te_nbt);
                let update_packet = UpdateBlockEntity {
                    block_pos: secret.block_pos,
                    action: 4, // 4 = skull update in 1.8
                    nbt_data: Some(nbt_bytes.clone()),
                };
                for (_, player) in &mut world.players {
                    player.write_packet(&update_packet);
                }
                let chunk_x = secret.block_pos.x >> 4;
                let chunk_z = secret.block_pos.z >> 4;
                if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                    chunk.packet_buffer.write_packet(&update_packet);
                }
                world.interactable_blocks.insert(secret.block_pos, BlockInteractAction::WitherEssence {
                    secret: secret_rc.clone()
                });
            }
        }
    }

    pub fn player_collides(&mut self, player: &Player) -> bool {
        player.collision_aabb().intersects(&self.spawn_aabb)
    }
}

/// Entity implementation for secret item drops
/// Handles item animation, player collision detection, and collection
pub struct SecretItemEntityImpl {
    secret: Rc<RefCell<DungeonSecret>>,
}

impl EntityImpl for SecretItemEntityImpl {
    fn spawn(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        // Set initial velocity to zero to prevent item from moving
        let velocity_packet = EntityVelocity {
            entity_id: VarInt(entity.id),
            velocity_x: 0,
            velocity_y: 0,
            velocity_z: 0,
        };
        buffer.write_packet(&velocity_packet);
    }
    
    fn tick(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        // After 20 ticks, adjust position to ensure item is on ground
        if entity.ticks_existed == 20 {
            entity.position.y -= 0.5;
        } else if entity.ticks_existed % 20 == 0 {
            // Re-sync position and velocity every second to prevent desync
            entity.last_position = DVec3::ZERO;
            
            let velocity_packet = EntityVelocity {
                entity_id: VarInt(entity.id),
                velocity_x: 0,
                velocity_y: 0,
                velocity_z: 0,
            };
            buffer.write_packet(&velocity_packet);
        }
        
        // 0.5s cooldown before item can be picked up (10 ticks)
        const PICKUP_COOLDOWN_TICKS: u32 = 10;
        if entity.ticks_existed < PICKUP_COOLDOWN_TICKS {
            return; // Item is still on cooldown, can't be picked up yet
        }
        
        // Check for player collision
        // Item pickup radius: 3 blocks in all directions
        const PICKUP_RADIUS: f64 = 3.0;
        let item_aabb = AABB::new(
            DVec3::new(
                entity.position.x - PICKUP_RADIUS,
                entity.position.y - PICKUP_RADIUS,
                entity.position.z - PICKUP_RADIUS,
            ),
            DVec3::new(
                entity.position.x + PICKUP_RADIUS,
                entity.position.y + PICKUP_RADIUS,
                entity.position.z + PICKUP_RADIUS,
            ),
        );
        
        let world = entity.world_mut();
        for player in world.players.values_mut() {
            if player.collision_aabb().intersects(&item_aabb) {
                // Mark secret as obtained
                {
                    let mut secret = self.secret.borrow_mut();
                    if !secret.obtained {
                        secret.obtained = true;
                    }
                }
                
                // Send collect item packet
                player.write_packet(&CollectItem {
                    item_entity_id: VarInt(entity.id),
                    entity_id: VarInt(player.entity_id),
                });
                
                // Play collection sound
                player.write_packet(&SoundEffect {
                    sound: "random.pop",
                    pos_x: player.position.x,
                    pos_y: player.position.y,
                    pos_z: player.position.z,
                    volume: 0.2,
                    pitch: 1.7619047,
                });
                
                // Despawn the entity
                world.despawn_entity(entity.id);
                return;
            }
        }
    }
}

// pub struct ItemSecretEntity;
// 
// // this isn't necessarily a secret, simply an animation for one, can be re-used for blessings
// impl EntityImpl for ItemSecretEntity {
//     fn spawn(&mut self, entity: &mut Entity, buffer: PacketBuffer) {
//         let metadata_packet = &PacketEntityMetadata {
//             entity_id: VarInt(entity.id),
//             metadata: entity.metadata.clone(),
//         };
//         let velocity_packet = &EntityVelocity {
//             entity_id: VarInt(entity.id),
//             velocity_x: 0,
//             velocity_y: 0,
//             velocity_z: 0,
//         };
//         for player in entity.world_mut().players.values_mut() {
//             player.write_packet(metadata_packet);
//             player.write_packet(velocity_packet);
//         }
//     }
//     
//     fn tick(&mut self, entity: &mut Entity) {
//         if entity.ticks_existed == 20 {
//             // this makes sure entity is on ground and prevents it jitter-ing in air
//             entity.position.y -= 0.5;
//         } else if entity.ticks_existed % 20 == 0 {
//             // re-sync position and velocity,
//             // since it is really easy for item drop to de-sync
//             entity.last_position = DVec3::ZERO;
// 
//             for player in entity.world_mut().players.values_mut() {
//                 player.write_packet(&EntityVelocity {
//                     entity_id: VarInt(entity.id),
//                     velocity_x: 0,
//                     velocity_y: 0,
//                     velocity_z: 0,
//                 });
//             }
//         }
//         
//         // todo get correct values
//         const W: f64 = 3.0;
//         const H: f64 = 3.0;
//         
//         let aabb = AABB::new(
//             DVec3::new(entity.position.x - W, entity.position.y - H, entity.position.z - W),
//             DVec3::new(entity.position.x + W, entity.position.y + H, entity.position.z + W),
//         );
//         for player in entity.world_mut().players.values_mut() {
//             if player.collision_aabb().intersects(&aabb) { 
//                 player.write_packet(&CollectItem {
//                     item_entity_id: VarInt(entity.id),
//                     entity_id: VarInt(player.entity_id),
//                 });
//                 player.write_packet(&SoundEffect {
//                     sound: "random.pop",
//                     pos_x: player.position.x,
//                     pos_y: player.position.y,
//                     pos_z: player.position.z,
//                     volume: 0.2,
//                     pitch: 1.7619047,
//                 });
//                 entity.world_mut().despawn_entity(entity.id);
//                 break;
//             }
//         }
//     }
//     
// }
// 
/// Entity implementation for wither essence animation
/// Creates a floating skull that rotates and plays particles/sounds
pub struct EssenceEntityImpl;

impl EntityImpl for EssenceEntityImpl {
    fn spawn(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        use std::collections::HashMap;
        
        // Create the skull item
        let skull_item = ItemStack {
            item: 397, // Player head
            stack_size: 1,
            metadata: 3,
            tag_compound: Some(NBT::with_nodes(vec![
                NBT::compound("SkullOwner", vec![
                    NBT::string("Name", ""),
                    NBT::string("Id", "e0f3e929-869e-3dca-9504-54c666ee6f23"),
                    NBT::compound("Properties", vec![
                        NBT::list("textures", TAG_COMPOUND_ID, vec![
                            NBTNode::Compound({
                                let mut map = HashMap::new();
                                map.insert("Value".into(), NBTNode::String("ewogICJ0aW1lc3RhbXAiIDogMTYwMzYxMDQ0MzU4MywKICAicHJvZmlsZUlkIiA6ICIzM2ViZDMyYmIzMzk0YWQ5YWM2NzBjOTZjNTQ5YmE3ZSIsCiAgInByb2ZpbGVOYW1lIiA6ICJEYW5ub0JhbmFubm9YRCIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9lNDllYzdkODJiMTQxNWFjYWUyMDU5Zjc4Y2QxZDE3NTRiOWRlOWIxOGNhNTlmNjA5MDI0YzRhZjg0M2Q0ZDI0IgogICAgfQogIH0KfQ==".to_string()));
                                map.insert("Signature".into(), NBTNode::String("Mnf7PDLe+FPiO+wQ2St6XNRiiIXtZ3GuPTcLlM7pNQ6d6MXuzI7xXG24qaAMFuVwMB+F3dLYcaFlc+bWyi3Qm9msSq2mMUXdvzTamAslZHcdcTFNpppkYgdvkOhWK7W/amQyd2Q+pLDECe8Mg6gxBY17+xfaWlIynzEWEmHR+ye+hTC44kgiTZaYiRg7gpU002deY8WpX875cc5zJIroxVR52qHIV+suIMPwq47mpCp520J9R1HuYvvP/V3+PwL7skMlC1F/HHkG5A13fvSKMqq9XMsdqXR8qvWlcL5IQTS7ijtD9TZo8jcmhz/7HCXuJ912I1GqJp4hZ0Lqa0NB0TuI/giHr2i4yNzORe6oan47bpMXLoZWIrYZIOsF6wSObhwniF1jM/zUEkum9XswRImIvYYlmyLH+Kkh5uQJm244rOLPXmOZEid6PW5bhaSRpMOMpxboeOtjLbGC56Ev+DwoI37SrAYY6/LC7HwjVhvkcsLd/9BrF+Wl10bdLdsJEbd+TII59/45MM1x7+xgeAFU/ip0TjkMPfRLdNmfxOGssMFZOaM55iOb+8t4tOvXxnqeXpFCByDgPnqKV5zPXS1XMF2+5qEAv7ZKrqK8BLAHbWsKHHOMt1hJ8K+EgYfRDKq72YvN01ST288ysUv8b5stRu8O5uC+KvZXtnlGrKc=".to_string()));
                                map
                            })
                        ])
                    ])
                ]),
            ])),
        };
        
        // Send equipment packet through buffer (for players in chunk)
        buffer.write_packet(&EntityEquipment {
            entity_id: VarInt(entity.id),
            item_slot: 4, // Helmet slot
            item_stack: Some(skull_item.clone()),
        });
        
        // Also send directly to all players to ensure it's received
        let world = entity.world_mut();
        for player in world.players.values_mut() {
            player.write_packet(&EntityEquipment {
                entity_id: VarInt(entity.id),
                item_slot: 4, // Helmet slot
                item_stack: Some(skull_item.clone()),
            });
        }
    }
    
    fn tick(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        // Make entity float up and rotate
        entity.position.y += 0.04;
        entity.yaw += 15.0;
        
        // Send position/rotation updates so the client sees the movement
        if entity.ticks_existed % 2 == 0 {
            // Send position update every 2 ticks
            let world = entity.world_mut();
            for player in world.players.values_mut() {
                player.write_packet(&EntityTeleport {
                    entity_id: entity.id,
                    pos_x: entity.position.x,
                    pos_y: entity.position.y,
                    pos_z: entity.position.z,
                    yaw: entity.yaw,
                    pitch: entity.pitch,
                    on_ground: false,
                });
            }
        }
        
        if entity.ticks_existed % 5 == 0 {
            // Spawn particles every 5 ticks (sounds are handled by scheduled sounds in block interaction)
            let particle_packet = Particles {
                particle_id: 29,
                long_distance: true,
                x: entity.position.x as f32,
                y: entity.position.y as f32 + 1.5,
                z: entity.position.z as f32,
                offset_x: 0.0,
                offset_y: 0.0,
                offset_z: 0.0,
                speed: 0.06,
                count: 5,
            };
            
            let world = entity.world_mut();
            for player in world.players.values_mut() {
                player.write_packet(&particle_packet);
            }
        }
        
        if entity.ticks_existed == 20 {
            // After 20 ticks, play orb sound twice and despawn
            let sound_packet = SoundEffect {
                sound: "random.orb",
                pos_x: entity.position.x,
                pos_y: entity.position.y,
                pos_z: entity.position.z,
                volume: 1.0,
                pitch: 1.5,
            };
            
            let world = entity.world_mut();
            for player in world.players.values_mut() {
                // Send twice (as per Hypixel behavior)
                player.write_packet(&sound_packet);
                player.write_packet(&sound_packet);
            }
            world.despawn_entity(entity.id);
        }
    }
}

/*
door success
[12:55:46] sound note.pling, 8.0 4.047619 -140.75 69.0 -154.75
[12:55:47]  Stivais opened a WITHER door!

door fail
[12:58:06] sound mob.endermen.portal, 8.0 0.0 -58.0 69.0 -171.75
[12:58:06]  You do not have the key for this door!
*/

/*
unrelated to secrets, but this is dungeon countdown
[12:43:22]  Stivais is now ready!
[12:43:22]  sound random.click, 0.55 2.0 -119.625 69.0 -173.375
[12:43:22]  Starting in 4 seconds.
[12:43:23]  sound random.click, 0.55 2.0 -117.75 69.0 -172.375
[12:43:23]  Starting in 3 seconds.
[12:43:24]  sound random.click, 0.55 2.0 -118.75 69.0 -173.0
[12:43:24]  Starting in 2 seconds.
[12:43:25]  sound random.click, 0.55 2.0 -118.75 69.0 -173.0
[12:43:25]  Starting in 1 second.
[12:43:26]  sound mob.enderdragon.growl, 1.0 1.0 -118.75 69.0 -173.0
*/

/*
[12:43:26]  sound mob.villager.haggle, 1.0 0.6984127 -118.75 69.0 -173.0
[12:43:26]  §e[NPC] §bMort§f: Here, I found this map when I first entered the dungeon.

[12:43:28]  sound mob.villager.haggle, 1.0 0.6984127 -117.875 69.0 -175.875
[12:43:28]  §e[NPC] §bMort§f: You should find it useful if you get lost.
[12:43:29]  sound mob.villager.haggle, 1.0 0.6984127 -110.875 71.0 -184.375
[12:43:29]  §e[NPC] §bMort§f: Good luck.

*/
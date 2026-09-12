use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityImpl, EntityId, NoEntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::{ClientId, Player};
use crate::server::utils::aabb::AABB;
use crate::server::utils::direction::Direction;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::nbt::nbt::{NBT, NBTNode};
use crate::server::utils::nbt::serialize::TAG_COMPOUND_ID;
use std::collections::HashMap;
use crate::server::world::World;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{CollectItem, EntityEquipment, EntityTeleport, EntityVelocity, Particles, SoundEffect};
use crate::server::utils::particles::ParticleTypes;
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
    /// `Some` only for the one chest that should also reveal a "blessing" on open - a floating
    /// skull wearing this base64 texture, plus the ascending `note.harp` sequence (see the
    /// `Chest` interact handler in `block_interact_action.rs`). `None` for every ordinary secret
    /// chest, which just does the normal open.
    pub blessing_texture: Option<&'static str>,
    /// `Some` only for a chest whose *own opening* is a puzzle's actual win condition (unlike,
    /// say, Creeper Beams, which completes the instant its last correct pair locks in, with no
    /// further interaction needed) - the Teleport Maze reward chest, confirmed: reaching it just
    /// reveals the chest, opening it is what actually solves the puzzle. When set, the `Chest`
    /// interact handler marks this room index `puzzle_completed` and broadcasts the solved
    /// message the first time this chest is opened, instead of any of that happening earlier.
    pub puzzle_room_index: Option<usize>,
    /// Shared countdown for a puzzle whose win condition is opening *multiple* specific chests
    /// (Ice Fill's two blessing chests, confirmed: opening both is the actual win condition, not
    /// just reaching/opening either one) rather than any single chest completing it outright.
    /// `Some`, pointing at the same counter, on every secret in the group - `puzzle_room_index`
    /// above only actually completes the room once this counter reaches 0 (decremented on each
    /// open in the `Chest` interact handler). `None` keeps today's behavior: `puzzle_room_index`
    /// completes the instant that one chest opens (Teleport Maze's single reward chest).
    pub puzzle_chest_group: Option<Rc<std::cell::Cell<u8>>>,
    /// Whether opening this chest increments the room's `found_secrets` count (see the `Chest`
    /// interact handler in `block_interact_action.rs`). `true` for every ordinary secret chest;
    /// `false` for a chest that's purely a bonus reward and isn't one of the room's actual
    /// counted secrets - e.g. Tic Tac Toe's reward chest (`tic_tac_toe.rs::reveal_reward_chest`),
    /// which already counts the puzzle itself as solved and shouldn't also bump the secret tally.
    pub counts_as_secret: bool,
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
            blessing_texture: None,
            puzzle_room_index: None,
            puzzle_chest_group: None,
            counts_as_secret: true,
        }
    }
    
    /// Single flame particle floating just above a chest that just spawned into the world -
    /// same effect/placement as the Three Weirdos puzzle's correct-chest tell
    /// (`three_weirdos::WeirdoImpl::interact`), applied here to every ordinary chest secret
    /// instead. `+1.0` on Y clears the chest's own (well-under-a-full-block) model so the
    /// particle floats visibly in open air above the lid, rather than landing inside the solid
    /// block mesh where it'd be hidden from the camera's point of view.
    fn spawn_chest_flame(world: &mut World, block_pos: BlockPos) {
        let particles = Particles {
            particle_id: ParticleTypes::Flame.get_id(),
            long_distance: true,
            x: block_pos.x as f32 + 0.5,
            y: block_pos.y as f32 + 1.0,
            z: block_pos.z as f32 + 0.5,
            offset_x: 0.0,
            offset_y: 0.0,
            offset_z: 0.0,
            speed: 0.0,
            count: 1,
        };
        for player in world.players.values_mut() {
            player.write_packet(&particles);
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
                Self::spawn_chest_flame(world, secret.block_pos);
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
                Self::spawn_chest_flame(world, secret.block_pos);
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
                Self::spawn_chest_flame(world, secret.block_pos);
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
            entity_id: entity.id,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
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
                entity_id: entity.id,
                velocity_x: 0.0,
                velocity_y: 0.0,
                velocity_z: 0.0,
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
/// Base64 skull texture `Value` the wither essence float animation's armor stand wears -
/// extracted here so it can be passed to `EssenceEntityImpl` like any other texture (see
/// `EssenceEntityImpl::texture`'s doc comment for why that struct is parameterized instead of
/// hardcoding this).
pub const WITHER_ESSENCE_TEXTURE: &str = "ewogICJ0aW1lc3RhbXAiIDogMTYwMzYxMDQ0MzU4MywKICAicHJvZmlsZUlkIiA6ICIzM2ViZDMyYmIzMzk0YWQ5YWM2NzBjOTZjNTQ5YmE3ZSIsCiAgInByb2ZpbGVOYW1lIiA6ICJEYW5ub0JhbmFubm9YRCIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9lNDllYzdkODJiMTQxNWFjYWUyMDU5Zjc4Y2QxZDE3NTRiOWRlOWIxOGNhNTlmNjA5MDI0YzRhZjg0M2Q0ZDI0IgogICAgfQogIH0KfQ==";

/// Entity implementation for a floating, spinning skull with particles - used for both the wither
/// essence pickup animation and (reusing this exact same effect, per this file's own left-behind
/// note next to the real captured `note.harp` sound sequence - see the `Chest` interact handler)
/// a "blessing" chest's reveal. `texture` is whichever base64 skull `Value` the armor stand's head
/// should show - `WITHER_ESSENCE_TEXTURE` for the original use, a real captured player skin for a
/// blessing.
pub struct EssenceEntityImpl {
    pub texture: &'static str,
    /// Tick (relative to spawn) this despawns at - 20 (1s) for the original wither essence use,
    /// 300 (15s) for a blessing, matching however long that particular use is meant to hover.
    /// A floating nametag, if wanted, is set on the entity's own `EntityMetadata` (`custom_name`/
    /// `custom_name_visible`) at the call site before `spawn_entity` instead of being a field
    /// here - metadata like that is already baked into the spawn packet by the time
    /// `EntityImpl::spawn` runs, so there's nothing for this struct itself to do with it.
    pub lifetime_ticks: u32,
    /// `Some` only for a real blessing chest's reveal (not the original wither-essence use) -
    /// which of the 4 blessing types this is, so the despawn-time "DUNGEON BUFF!" chat
    /// announcement (see `dungeon::blessings::format_pickup_message`) knows what to say. The
    /// floating nametag shown for the full 15s hover doesn't include a level (matches the
    /// pre-existing, unchanged behavior at the call site) - only the chat message does, computed
    /// fresh right when it's actually granted.
    pub blessing: Option<crate::dungeon::blessings::BlessingKind>,
}

impl EntityImpl for EssenceEntityImpl {
    fn spawn(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        // Same unsigned-skull convention as `PickupKind::equipped_item` (Wither/Blood key heads) -
        // no `Signature` needed for a custom texture to render, only for Mojang session-server
        // verification, which a private server doesn't do.
        let mut skull_item = ItemStack {
            item: 397, // Player head
            stack_size: 1,
            metadata: 3,
            tag_compound: None,
        };
        skull_item.set_skull_owner(self.texture);

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
        // Rise for the first 20 ticks only (matches the original 1s wither-essence animation's
        // total rise, ~0.8 blocks) - for a 15s blessing (`lifetime_ticks` 300), continuing this
        // same per-tick rise for the *entire* duration would carry it ~12 blocks up through the
        // ceiling. It just hovers in place (still rotating) for whatever's left after that.
        if entity.ticks_existed < 20 {
            entity.position.y += 0.04;
        }
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
        
        if entity.ticks_existed == self.lifetime_ticks {
            // After `lifetime_ticks`, play orb sound twice and despawn
            let sound_packet = SoundEffect {
                sound: "random.orb",
                pos_x: entity.position.x,
                pos_y: entity.position.y,
                pos_z: entity.position.z,
                volume: 1.0,
                pitch: 1.5,
            };

            let world = entity.world_mut();

            // The blessing is actually granted/announced right here, at a fixed level (no
            // per-run level tracking - see `dungeon::blessings`'s own doc comment).
            if let Some(kind) = self.blessing {
                let lines = crate::dungeon::blessings::format_pickup_message(kind, crate::dungeon::blessings::FIXED_LEVEL);
                for player in world.players.values_mut() {
                    player.send_message(&lines[0]);
                    player.send_message(&lines[1]);
                }
            }

            for player in world.players.values_mut() {
                // Send twice (as per Hypixel behavior)
                player.write_packet(&sound_packet);
                player.write_packet(&sound_packet);
            }
            world.despawn_entity(entity.id);
        }
    }
}

/// What a `PickupEntityImpl` gives the player: a Wither/Blood Door key (a boolean flag, not a
/// real inventory item), Superboom TNT (a real stackable inventory item), or a random Dungeon
/// Blessing (see `dungeon::blessings`). Wither/Blood also double as which `DoorType` they gate -
/// the rest aren't tied to any door.
///
/// `Blessing` is granted on room clear alongside a guaranteed `Tnt` (see
/// `Dungeon::grant_room_clear_rewards`) - "like how wither keys drop on last mob" - reusing this
/// same walk-up-and-touch pickup mechanism rather than the timer-based `EssenceEntityImpl` a
/// blessing chest's own reveal uses, since that's specifically what was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickupKind {
    Wither,
    Blood,
    Tnt,
    Blessing(crate::dungeon::blessings::BlessingKind),
}

impl PickupKind {
    /// `None` for everything except the two real door keys - see the type-level doc comment.
    pub fn door_type(self) -> Option<crate::dungeon::door::DoorType> {
        match self {
            PickupKind::Wither => Some(crate::dungeon::door::DoorType::WITHER),
            PickupKind::Blood => Some(crate::dungeon::door::DoorType::BLOOD),
            PickupKind::Tnt | PickupKind::Blessing(_) => None,
        }
    }

    /// Floating nametag text, and also what's used mid-sentence in the "has obtained" message
    /// below (both happen to want the same colored name). Owned rather than `&'static str`
    /// only because `Blessing`'s name depends on which of the 4 types it randomly picked.
    pub fn colored_name(self) -> String {
        match self {
            PickupKind::Wither => "\u{a7}8Wither Key".to_string(),
            PickupKind::Blood => "\u{a7}cBlood Key".to_string(),
            PickupKind::Tnt => "\u{a7}9Superboom TNT".to_string(),
            PickupKind::Blessing(kind) => format!("\u{a7}dBlessing of {}", kind.display_name()),
        }
    }

    /// Personal follow-up lines sent only to the picker, not broadcast - empty for anything that
    /// doesn't need "how to use this" instructions (only the two real keys do).
    ///
    /// Each kind is a single combined string, not two - Skytils' `DungeonListener.keyPickupRegex`
    /// (`§r§e§lRIGHT CLICK §r§7on §r§7.+?§r§7 to open it\. This key can only be used to open
    /// §r§a(?<num>\d+)§r§7 door!§r`, confirmed from Skytils' own source) matches against one
    /// packet's `formattedText` as a whole - this used to be sent as two separate
    /// `player.send_message` calls (two packets), so neither one alone ever contained the full
    /// "RIGHT CLICK...to open it. This key can...door!" span the regex needs, and Skytils'
    /// `DungeonInfo.keys` counter (which drives the "you have a key, door outline turns green"
    /// state) never incremented. Also needed the `§r` reset codes the regex requires at each
    /// style change - real Hypixel's message is built from styled chat-component siblings, which
    /// the client's `getFormattedText()` auto-inserts `§r` between; a flat legacy-code string has
    /// to add those explicitly to reproduce the exact same text.
    fn extra_messages(self) -> &'static [&'static str] {
        match self {
            PickupKind::Wither => &[
                "\u{a7}r\u{a7}e\u{a7}lRIGHT CLICK \u{a7}r\u{a7}7on \u{a7}r\u{a7}7a \u{a7}8Wither Door\u{a7}r\u{a7}7 to open it. This key can only be used to open \u{a7}r\u{a7}a1\u{a7}r\u{a7}7 door!\u{a7}r",
            ],
            PickupKind::Blood => &[
                "\u{a7}r\u{a7}e\u{a7}lRIGHT CLICK \u{a7}r\u{a7}7on \u{a7}r\u{a7}7the \u{a7}cBLOOD DOOR\u{a7}r\u{a7}7 to open it. This key can only be used to open \u{a7}r\u{a7}a1\u{a7}r\u{a7}7 door!\u{a7}r",
            ],
            PickupKind::Tnt | PickupKind::Blessing(_) => &[],
        }
    }

    /// Bare base64 `Value` (no `Signature`) for `ItemStack::set_skull_owner` - same unsigned-skull
    /// convention already used for the Mimic/Sniper/Fels heads in `spawner.rs`. Never called for
    /// `Tnt`, which has no skull at all - see `equipped_item` below.
    fn skull_texture(self) -> &'static str {
        match self {
            PickupKind::Wither => "ewogICJ0aW1lc3RhbXAiIDogMTYwMzYxMDQ0MzU4MywKICAicHJvZmlsZUlkIiA6ICIzM2ViZDMyYmIzMzk0YWQ5YWM2NzBjOTZjNTQ5YmE3ZSIsCiAgInByb2ZpbGVOYW1lIiA6ICJEYW5ub0JhbmFubm9YRCIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9lNDllYzdkODJiMTQxNWFjYWUyMDU5Zjc4Y2QxZDE3NTRiOWRlOWIxOGNhNTlmNjA5MDI0YzRhZjg0M2Q0ZDI0IgogICAgfQogIH0KfQ==",
            PickupKind::Blood => "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvNmU1Y2Y3ZjJlMGY2YjE2N2IwYjZmZDBjNGFjMTZjYTcwZTRjNWM4MTFiOGQ1YWQwZWVkMmUzYWE2ZGQyYjcifX19",
            // Same shared texture the blessing-chest reveal itself uses for all 4 types
            // (`REWARD_BLESSING_TEXTURE` in `boulder.rs`/`ice_fill.rs`/`tic_tac_toe.rs`/
            // `ice_path.rs`/`teleport_maze.rs`, each keeping its own copy rather than importing a
            // shared one - following that same convention here) - only the nametag varies per
            // type, not the skin.
            PickupKind::Blessing(_) => "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZTkzZTIwNjg2MTc4NzJjNTQyZWNkYTFkMjdkZjRlY2U5MWM2OTk5MDdiZjMyN2M0ZGRiODUzMDk0MTJkMzkzOSJ9fX0=",
            PickupKind::Tnt => unreachable!("Tnt has no skull texture"),
        }
    }

    /// What the floating armor stand wears in its helmet slot.
    fn equipped_item(self) -> ItemStack {
        match self {
            PickupKind::Wither | PickupKind::Blood | PickupKind::Blessing(_) => {
                let mut skull = ItemStack {
                    item: 397, // Player head
                    stack_size: 1,
                    metadata: 3,
                    tag_compound: None,
                };
                skull.set_skull_owner(self.skull_texture());
                skull
            }
            PickupKind::Tnt => {
                use crate::server::items::item_stack::{Enchant, ItemStackExt};
                // Enchanted purely for the glint - matches `Item::SuperboomTNT`'s real item, but
                // skips its lore/rarity NBT since nobody opens a tooltip on a floating pickup.
                ItemStack::new(46).ench(Enchant::Sharpness, 1)
            }
        }
    }

    /// Applies the actual effect to `player` - a boolean flag for either key. TNT and Blessing
    /// are both purely cosmetic props with no direct player-state effect: TNT has nothing to
    /// grant at all, and a blessing's real "effect" is the DUNGEON BUFF! chat announcement (this
    /// project has no stat system to actually apply a buff to), handled as a special case in
    /// `PickupEntityImpl::tick` instead of here.
    fn apply(self, player: &mut Player) {
        match self {
            PickupKind::Wither => player.has_wither_key = true,
            PickupKind::Blood => player.has_blood_key = true,
            PickupKind::Tnt | PickupKind::Blessing(_) => {}
        }
    }
}

/// Not a `DungeonSecret` (nothing in `secrets.json` describes any of these) - spawned
/// dynamically by `Dungeon::maybe_grant_door_key` (see `dungeon.rs`) whenever the room leading to
/// a Wither or Blood Door has no starred mobs left, and picked up by simple proximity like
/// `SecretItemEntityImpl` rather than following the `DungeonSecret`/obtained-flag flow. Lives
/// here anyway since it reuses `EssenceEntityImpl`'s equipment approach (an item worn by an
/// invisible armor stand) and this file already has all the NBT/AABB imports.
pub struct PickupEntityImpl {
    pub kind: PickupKind,
}

impl EntityImpl for PickupEntityImpl {
    fn spawn(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        let item = self.kind.equipped_item();

        buffer.write_packet(&EntityEquipment {
            entity_id: VarInt(entity.id),
            item_slot: 4, // Helmet slot
            item_stack: Some(item.clone()),
        });

        let world = entity.world_mut();
        for player in world.players.values_mut() {
            player.write_packet(&EntityEquipment {
                entity_id: VarInt(entity.id),
                item_slot: 4,
                item_stack: Some(item.clone()),
            });
        }
    }

    fn tick(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        // Gentle spin in place (no upward drift, unlike the essence - this has to stay reachable
        // until someone walks up and grabs it, however long that takes).
        entity.yaw += 6.0;
        if entity.ticks_existed % 2 == 0 {
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

        // Avoids an instant, feedback-less pickup if a player is already standing right where
        // the last starred mob died.
        const PICKUP_COOLDOWN_TICKS: u32 = 5;
        if entity.ticks_existed < PICKUP_COOLDOWN_TICKS {
            return;
        }

        const PICKUP_RADIUS: f64 = 2.0;
        // After 10s (200 ticks) nobody's grabbed it, hand it to whoever's closest instead of
        // leaving it floating forever behind a range this tight.
        const AUTO_PICKUP_TICKS: u32 = 200;

        let world = entity.world_mut();

        let picked_up_by: Option<ClientId> = if entity.ticks_existed >= AUTO_PICKUP_TICKS {
            world.players.iter()
                .min_by(|(_, a), (_, b)| {
                    a.position.distance_squared(&entity.position)
                        .total_cmp(&b.position.distance_squared(&entity.position))
                })
                .map(|(id, _)| *id)
        } else {
            let key_aabb = AABB::new(
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
            world.players.iter()
                .find(|(_, player)| player.collision_aabb().intersects(&key_aabb))
                .map(|(id, _)| *id)
        };

        let Some(player_id) = picked_up_by else { return; };
        let Some(player) = world.players.get_mut(&player_id) else { return; };

        self.kind.apply(player);

        // No `CollectItem` packet here, per explicit correction - that packet is what makes the
        // vanilla client animate the item visibly flying to the player over the next few ticks;
        // a wither/blood key, TNT, or blessing pickup is a real, instant grab with no such flight,
        // confirmed real. `entity.item.pickup`'s legacy equivalent, at the real captured
        // volume/pitch (not `SecretItemEntityImpl`'s quieter/higher-pitched "plop" - that one's
        // unrelated and untouched).
        player.write_packet(&SoundEffect {
            sound: "random.pop",
            pos_x: player.position.x,
            pos_y: player.position.y,
            pos_z: player.position.z,
            volume: 1.0,
            pitch: 1.0,
        });

        let kind = self.kind;
        if let PickupKind::Blessing(blessing_kind) = kind {
            // Its own real "DUNGEON BUFF!" 2-line announcement, not the generic "has obtained"
            // line every other kind uses - see `dungeon::blessings::format_pickup_message`.
            let lines = crate::dungeon::blessings::format_pickup_message(blessing_kind, crate::dungeon::blessings::FIXED_LEVEL);
            for other_player in world.players.values_mut() {
                other_player.send_message(&lines[0]);
                other_player.send_message(&lines[1]);
            }
        } else {
            // Per explicit correction: Superboom TNT gets the same "has obtained" message as
            // every other non-Blessing pickup - it isn't silent just because it's cosmetic.
            let username = player.profile.username.clone();
            for other_player in world.players.values_mut() {
                other_player.send_message(&format!("\u{a7}a{} \u{a7}ehas obtained {}\u{a7}e!", username, kind.colored_name()));
            }

            if let Some(player) = world.players.get_mut(&player_id) {
                for line in kind.extra_messages() {
                    player.send_message(line);
                }
            }
        }

        world.despawn_entity(entity.id);
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
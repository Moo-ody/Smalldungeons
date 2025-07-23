use crate::net::packets::client_bound::collect_item::PacketCollectItem;
use crate::net::packets::client_bound::entity::entity_equipment::EntityEquipment;
use crate::net::packets::client_bound::entity::entity_metadata::PacketEntityMetadata;
use crate::net::packets::client_bound::entity::entity_velocity::EntityVelocity;
use crate::net::packets::client_bound::particles::Particles;
use crate::net::packets::client_bound::sound_effect::SoundEffect;
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::utils::aabb::AABB;
use crate::server::utils::direction::Direction;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::nbt::encode::TAG_COMPOUND_ID;
use crate::server::utils::nbt::{NBTNode, NBT};
use crate::server::utils::particles::ParticleTypes;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;

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
}

// todo new function that infers spawn_aabb from a range
pub struct DungeonSecret {
    pub secret_type: SecretType,
    pub block_pos: BlockPos,
    pub spawn_aabb: AABB,
    pub has_spawned: bool,
    pub obtained: bool,
}

// when this is integrated into rooms, remove this and just inline the spawning stuff
pub fn tick(dungeon_secret: &Rc<RefCell<DungeonSecret>>, player: &Player) {
    let mut secret = dungeon_secret.borrow_mut();
    if !secret.has_spawned && player.collision_aabb().intersects(&secret.spawn_aabb) {
        secret.has_spawned = true;
        DungeonSecret::spawn(dungeon_secret, secret, player.world_mut())
    }
}

impl DungeonSecret {

    fn spawn(dungeon_secret: &Rc<RefCell<DungeonSecret>>, secret: RefMut<DungeonSecret>, world: &mut World) {
        match &secret.secret_type {
            SecretType::WitherEssence { .. } => {
                world.set_block_at(
                    Blocks::Skull { direction: Direction::Up, no_drop: false },
                    secret.block_pos.x,
                    secret.block_pos.y,
                    secret.block_pos.z
                );
                world.interactable_blocks.insert(secret.block_pos, BlockInteractAction::WitherEssence {
                    secret: dungeon_secret.clone()
                });
            }
            SecretType::Chest { direction } => {
                world.set_block_at(
                    Blocks::Chest { direction: direction.clone() },
                    secret.block_pos.x,
                    secret.block_pos.y,
                    secret.block_pos.z,
                );
                world.interactable_blocks.insert(secret.block_pos, BlockInteractAction::Chest {
                    secret: dungeon_secret.clone()
                });
            }
            SecretType::Item { item } => {
                world.spawn_entity(
                    DVec3::from_centered(&secret.block_pos),
                    EntityMetadata::new(EntityVariant::DroppedItem {
                        item: item.clone()
                    }),
                    ItemSecretEntity,
                ).unwrap();
            }
            SecretType::Bat => {}
        }
    }

    pub fn player_collides(&mut self, player: &Player) -> bool {
        player.collision_aabb().intersects(&self.spawn_aabb)
    }
}

pub struct ItemSecretEntity;

// todo: implement some form of packet buffer, which writes packets and flushes once

// this isn't necessarily a secret, simply an animation for one, can be re-used for blessings
impl EntityImpl for ItemSecretEntity {
    fn spawn(&mut self, entity: &mut Entity) {
        for player in entity.world_mut().players.values() {
            player.send_packet(PacketEntityMetadata {
                entity_id: entity.id,
                metadata: entity.metadata.clone()
            }).unwrap();
            player.send_packet(EntityVelocity {
                entity_id: entity.id,
                motion_x: 0.0,
                motion_y: 0.2,
                motion_z: 0.0,
            }).unwrap();
        }
    }
    
    fn tick(&mut self, entity: &mut Entity) {
        if entity.ticks_existed == 20 {
            // this makes sure entity is on ground and prevents it jitter-ing in air
            entity.position.y -= 0.5;
        } else if entity.ticks_existed % 20 == 0 {
            // re-sync position and velocity,
            // since it is really easy for item drop to de-sync
            entity.last_position = DVec3::ZERO;

            for player in entity.world_mut().players.values() {
                player.send_packet(EntityVelocity {
                    entity_id: entity.id,
                    motion_x: 0.0,
                    motion_y: 0.0,
                    motion_z: 0.0,
                }).unwrap();
            }
        }
        
        // todo get correct values
        const W: f64 = 1.5;
        const H: f64 = 1.5;
        
        let aabb = AABB::new(
            DVec3::new(entity.position.x - W, entity.position.y - H, entity.position.z - W),
            DVec3::new(entity.position.x + W, entity.position.y + H, entity.position.z + W),
        );
        for player in entity.world_mut().players.values() {
            if player.collision_aabb().intersects(&aabb) { 
                player.send_packet(PacketCollectItem {
                    item_entity_id: entity.id,
                    entity_id: player.entity_id,
                }).unwrap();
                player.send_packet(SoundEffect {
                    sounds: Sounds::Pop,
                    volume: 0.2,
                    pitch: 1.7619047,
                    x: player.position.x,
                    y: player.position.y,
                    z: player.position.z,
                }).unwrap();
                entity.world_mut().despawn_entity(entity.id);
                break;
            }
        }
    }
    
}

pub struct EssenceEntity;

impl EntityImpl for EssenceEntity {
    
    fn spawn(&mut self, entity: &mut Entity) {
        for player in entity.world_mut().players.values() {
            player.send_packet(EntityEquipment {
                entity_id: entity.id,
                item_slot: 4,
                item_stack: Some(ItemStack {
                    item: 397,
                    stack_size: 1,
                    metadata: 3,
                    tag_compound: Some(NBT::with_nodes(vec![
                        NBT::compound("SkullOwner", vec![
                            NBT::string("Name", ""),
                            NBT::string("Id", "e0f3e929-869e-3dca-9504-54c666ee6f23"),
                            NBT::compound("Properties", vec![
                                NBT::list("textures", TAG_COMPOUND_ID,vec![
                                    NBTNode::Compound(vec![
                                        NBT::string("Value", "ewogICJ0aW1lc3RhbXAiIDogMTYwMzYxMDQ0MzU4MywKICAicHJvZmlsZUlkIiA6ICIzM2ViZDMyYmIzMzk0YWQ5YWM2NzBjOTZjNTQ5YmE3ZSIsCiAgInByb2ZpbGVOYW1lIiA6ICJEYW5ub0JhbmFubm9YRCIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9lNDllYzdkODJiMTQxNWFjYWUyMDU5Zjc4Y2QxZDE3NTRiOWRlOWIxOGNhNTlmNjA5MDI0YzRhZjg0M2Q0ZDI0IgogICAgfQogIH0KfQ=="),
                                        NBT::string("Signature", "Mnf7PDLe+FPiO+wQ2St6XNRiiIXtZ3GuPTcLlM7pNQ6d6MXuzI7xXG24qaAMFuVwMB+F3dLYcaFlc+bWyi3Qm9msSq2mMUXdvzTamAslZHcdcTFNpppkYgdvkOhWK7W/amQyd2Q+pLDECe8Mg6gxBY17+xfaWlIynzEWEmHR+ye+hTC44kgiTZaYiRg7gpU002deY8WpX875cc5zJIroxVR52qHIV+suIMPwq47mpCp520J9R1HuYvvP/V3+PwL7skMlC1F/HHkG5A13fvSKMqq9XMsdqXR8qvWlcL5IQTS7ijtD9TZo8jcmhz/7HCXuJ912I1GqJp4hZ0Lqa0NB0TuI/giHr2i4yNzORe6oan47bpMXLoZWIrYZIOsF6wSObhwniF1jM/zUEkum9XswRImIvYYlmyLH+Kkh5uQJm244rOLPXmOZEid6PW5bhaSRpMOMpxboeOtjLbGC56Ev+DwoI37SrAYY6/LC7HwjVhvkcsLd/9BrF+Wl10bdLdsJEbd+TII59/45MM1x7+xgeAFU/ip0TjkMPfRLdNmfxOGssMFZOaM55iOb+8t4tOvXxnqeXpFCByDgPnqKV5zPXS1XMF2+5qEAv7ZKrqK8BLAHbWsKHHOMt1hJ8K+EgYfRDKq72YvN01ST288ysUv8b5stRu8O5uC+KvZXtnlGrKc="),
                                    ])
                                ])
                            ])
                        ]),
                    ])),
                }),
            }).unwrap()
        }
    }
    
    fn tick(&mut self, entity: &mut Entity) {
        entity.position.y += 0.05;
        entity.yaw += 15.0;
        
        if entity.ticks_existed % 5 == 0 {
            for player in entity.world_mut().players.values() {
                player.send_packet(Particles::new(
                    ParticleTypes::Cloud,
                    entity.position.clone().add_y(1.5),
                    DVec3::ZERO,
                    0.06,
                    5,
                    true,
                    None,
                ).unwrap()).unwrap();
                
                player.send_packet(SoundEffect {
                    sounds: Sounds::Harp,
                    volume: 1.0,
                    pitch: 0.8 + ((entity.ticks_existed / 5) as f32 * 0.1),
                    x: entity.position.x,
                    y: entity.position.y + 1.5,
                    z: entity.position.z,
                }).unwrap()
            }
        }
        
        if entity.ticks_existed == 20 {
            for player in entity.world_mut().players.values() {
                player.send_packet(SoundEffect {
                    sounds: Sounds::Orb,
                    volume: 1.0,
                    pitch: 1.5,
                    x: entity.position.x,
                    y: entity.position.y,
                    z: entity.position.z,
                }).unwrap();
                player.send_packet(SoundEffect {
                    sounds: Sounds::Orb,
                    volume: 1.0,
                    pitch: 1.5,
                    x: entity.position.x,
                    y: entity.position.y,
                    z: entity.position.z,
                }).unwrap();
            }
            entity.world_mut().despawn_entity(entity.id);
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
use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::room::secrets::{DungeonSecret, EssenceEntity};
use crate::net::packets::client_bound::block_action::PacketBlockAction;
use crate::net::packets::client_bound::sound_effect::SoundEffect;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use std::cell::RefCell;
use std::rc::Rc;

pub enum BlockInteractAction {
    WitherDoor {
        door_index: usize
    },
    BloodDoor {
        door_index: usize
    },
    Chest {
        secret: Rc<RefCell<DungeonSecret>>,
    },
    WitherEssence {
        secret: Rc<RefCell<DungeonSecret>>
    },
    Lever,
    // mainly for quick debug,
    Callback(fn(&Player, &BlockPos)),
}


impl BlockInteractAction {
    pub fn interact(&self, player: &Player, block_pos: &BlockPos) {
        match self {
            Self::WitherDoor { door_index: id } => {
                // Play wither door opening sound effect
                player.send_packet(SoundEffect {
                    sounds: Sounds::NotePling,
                    volume: 8.0,
                    pitch: 4.05,
                    x: block_pos.x as f64,
                    y: block_pos.y as f64,
                    z: block_pos.z as f64,
                }).unwrap();

                let world = &mut player.server_mut().world;
                let dungeon = &mut player.server_mut().dungeon;

                if let DungeonState::Started { .. } = dungeon.state {
                    // todo check if player has a key
                    let door = &dungeon.doors[*id];
                    door.open_door(world);
                    
                    // Send message to all players when WITHER door is opened
                    let message = format!("§b{} §aopened a §8WITHER §adoor!", player.profile.username);
                    for (_, other_player) in &player.server_mut().world.players {
                        let _ = other_player.send_msg(&message);
                    }
                }
            }

            Self::BloodDoor { door_index: id } => {
                // Play blood door opening sound effect (ghast scream)
                player.send_packet(SoundEffect {
                    sounds: Sounds::GuardianScream,
                    volume: 2.0,
                    pitch: 0.49,
                    x: block_pos.x as f64,
                    y: block_pos.y as f64,
                    z: block_pos.z as f64,
                }).unwrap();

                let world = &mut player.server_mut().world;
                let dungeon = &mut player.server_mut().dungeon;

                if let DungeonState::Started { .. } = dungeon.state {
                    // todo check if player has a key
                    let door = &dungeon.doors[*id];
                    door.open_door(world);
                    
                    // Send message to all players when BLOOD door is opened
                    for (_, other_player) in &player.server_mut().world.players {
                        let _ = other_player.send_msg("§cThe §c§lBLOOD DOOR §chas been opened!");
                        let _ = other_player.send_msg("§5A shiver runs down your spine...");
                    }
                }
            }

            Self::Chest { secret } => {
                let mut secret = secret.borrow_mut();
                if !secret.obtained {
                    let p = PacketBlockAction {
                        block_pos: block_pos.clone(),
                        event_id: 1,
                        event_data: 1,
                        block_id: 54,
                    };
                    player.send_packet(p).unwrap();
                    player.send_packet(SoundEffect {
                        sounds: Sounds::ChestOpen,
                        volume: 1.0,
                        pitch: 0.975,
                        x: block_pos.x as f64,
                        y: block_pos.y as f64,
                        z: block_pos.z as f64,
                    }).unwrap();

                    secret.obtained = true;
                }

                pub enum BlockInteractAction {
    // ...existing variants...
    ThreeWeirdosNpc { id: u32, idx: u8 },
    ThreeWeirdosChest { id: u32, idx: u8 },
}



                /*
[19:37:29] sound random.chestopen, 0.5 0.9206349 -94.0 82.0 -51.0
[19:37:29] sound random.chestopen, 0.5 0.984127 -93.5 82.5 -50.5

// if its a blessing, can likely re-use wither essence as these values appear to be the same

[19:37:29] sound note.harp, 1.0 0.7936508 -94.0 82.0 -51.0
[19:37:29] sound note.harp, 1.0 0.8888889 -94.0 82.0 -51.0
[19:37:30] sound note.harp, 1.0 1.0 -94.0 82.0 -51.0
[19:37:30] sound note.harp, 1.0 1.0952381 -94.0 82.0 -51.0
[19:37:30] sound note.harp, 1.0 1.1904762 -94.0 82.0 -51.0
                */
                // player.send_msg("hi").unwrap();
            }
            
            Self::WitherEssence { secret } => {
                let mut secret = secret.borrow_mut();
                debug_assert!(!secret.obtained);

                let world = player.world_mut();

                world.set_block_at(Blocks::Air, block_pos.x, block_pos.y, block_pos.z);
                world.interactable_blocks.remove(block_pos);

                world.spawn_entity(
                    DVec3::from(block_pos).add_x(0.5).add_y(-1.4).add_z(0.5),
                    EntityMetadata {
                        variant: EntityVariant::ArmorStand,
                        is_invisible: true,
                    },
                    EssenceEntity,
                ).unwrap();

                secret.obtained = true;
            }
            
            Self::Lever => {
                // Send chat message about hearing something opening
                player.send_msg("§cYou hear the sound of something").unwrap();
                player.send_msg("§copening...").unwrap();
                
                // Play anvil break sound effect
                player.send_packet(SoundEffect {
                    sounds: Sounds::AnvilBreak,
                    volume: 1.0,
                    pitch: 1.7,
                    x: block_pos.x as f64,
                    y: block_pos.y as f64,
                    z: block_pos.z as f64,
                }).unwrap();
                
                // Remove this lever from interactable blocks so it can only be used once
                let world = &mut player.server_mut().world;
                world.interactable_blocks.remove(block_pos);
            }
            
            Self::Callback(func) => {
                func(player, block_pos);
            }
        }
    }
}


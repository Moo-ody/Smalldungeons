use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::room::secrets::DungeonSecret;
use crate::net::protocol::play::clientbound::{BlockAction, SoundEffect};
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::NoEntityImpl;
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use std::cell::RefCell;
use std::rc::Rc;
// use std::collections::HashMap;

use crate::server::world::ScheduledSound;
use crate::server::world::tactical_insertion::TacticalInsertionMarker;

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
    // Mushroom secret: bottom mushrooms (start) and top mushrooms (return nodes)
    MushroomBottom {
        set_index: usize,
    },
    MushroomTop,
    // mainly for quick debug,
    Callback(fn(&Player, &BlockPos)),
}


impl BlockInteractAction {
    pub fn interact(&self, player: &mut Player, block_pos: &BlockPos) {
        match self {
            Self::WitherDoor { door_index: id } => {
                // Play wither door opening sound effect
                let _ = player.write_packet(&SoundEffect {
                    sound: Sounds::NotePling.id(),
                    volume: 8.0,
                    pitch: 4.05,
                    pos_x: block_pos.x as f64,
                    pos_y: block_pos.y as f64,
                    pos_z: block_pos.z as f64,
                });

                let world = &mut player.server_mut().world;
                let dungeon = &mut player.server_mut().dungeon;

                if let DungeonState::Started { .. } = dungeon.state {
                    // todo check if player has key
                    let door = &dungeon.doors[*id];
                    door.open_door(world);
                    
                    // Send message to all players when WITHER door is opened
                    let message = format!("§b{} §aopened a §8§lWITHER §adoor!", player.profile.username);
                    for (_, other_player) in &mut player.server_mut().world.players {
                        let _ = other_player.send_message(&message);
                    }
                }
            }

            Self::BloodDoor { door_index: id } => {
                // Play blood door opening sound effect (ghast scream)
                let _ = player.write_packet(&SoundEffect {
                    sound: Sounds::GhastScream.id(),
                    volume: 2.0,
                    pitch: 0.49,
                    pos_x: block_pos.x as f64,
                    pos_y: block_pos.y as f64,
                    pos_z: block_pos.z as f64,
                });

                let world = &mut player.server_mut().world;
                let dungeon = &mut player.server_mut().dungeon;

                if let DungeonState::Started { .. } = dungeon.state {
                    // todo check if player has key
                    let door = &dungeon.doors[*id];
                    door.open_door(world);
                    
                    // Send message to all players when BLOOD door is opened
                    for (_, other_player) in &mut player.server_mut().world.players {
                        let _ = other_player.send_message("§cThe §c§lBLOOD DOOR §chas been opened!");
                        let _ = other_player.send_message("§5A shiver runs down your spine...");
                    }
                }
            }

            Self::Chest { secret } => {
                let mut secret = secret.borrow_mut();
                if !secret.obtained {
                    // maybe make this a packet where it is sent to all players
                    player.write_packet(&BlockAction {
                        block_pos: block_pos.clone(),
                        event_id: 1,
                        event_data: 1,
                        block_id: 54,
                    });
                    player.write_packet(&SoundEffect {
                        sound: "random.chestopen",
                        pos_x: block_pos.x as f64,
                        pos_y: block_pos.y as f64,
                        pos_z: block_pos.z as f64,
                        volume: 1.0,
                        pitch: 0.975,
                    });
                    secret.obtained = true;
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
            
            Self::MushroomBottom { set_index } => {
                // Debounce if already active
                if player.server_mut().world.tactical_insertions.iter().any(|(m, _)| m.client_id == player.client_id) {
                    return;
                }
                // Save precise origin and schedule return in 5s (100 ticks)
                let origin = player.position;
                let yaw = player.yaw;
                let pitch = player.pitch;

                // Teleport immediately to the corresponding UP point (center) by resolving within the player's current room
                {
                    let dungeon = &mut player.server_mut().dungeon;
                    if let Some(room_index) = dungeon.get_player_room(player) {
                        let room_ref = &dungeon.rooms[room_index];
                        if let Some(set) = room_ref.mushroom_sets.iter().find(|s| s.bottom.iter().any(|bp| bp == block_pos)) {
                            if let Some(dest) = set.up.get(0) {
                                player.write_packet(&crate::net::protocol::play::clientbound::PositionLook {
                                    x: dest.x as f64 + 0.5,
                                    y: dest.y as f64,
                                    z: dest.z as f64 + 0.5,
                                    yaw,
                                    pitch,
                                    flags: 0,
                                });
                            }
                        }
                    }
                }

                let return_tick = player.world_mut().tick_count + 100;
                let marker = TacticalInsertionMarker {
                    client_id: player.client_id,
                    return_tick,
                    origin,
                    damage_echo_window_ticks: 0,
                    yaw,
                    pitch,
                };
                // Optional cooldown sound pattern could be added to Vec
                player.world_mut().tactical_insertions.push((marker, Vec::<ScheduledSound>::new()));
            }

            Self::MushroomTop => {
                // Only valid if active
                let world = player.world_mut();
                if let Some(idx) = world.tactical_insertions.iter().position(|(m, _)| m.client_id == player.client_id) {
                    let (marker, _) = world.tactical_insertions.remove(idx);
                    // Teleport immediately to origin and keep yaw/pitch
                    player.write_packet(&crate::net::protocol::play::clientbound::PositionLook {
                        x: marker.origin.x,
                        y: marker.origin.y,
                        z: marker.origin.z,
                        yaw: marker.yaw,
                        pitch: marker.pitch,
                        flags: 0,
                    });
                }
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
                    NoEntityImpl,
                ).unwrap();

                secret.obtained = true;
            }
            
            Self::Lever => {
                // Send chat message about hearing something opening
                player.send_message("§cYou hear the sound of something");
                player.send_message("§copening...");
                
                // Play anvil break sound effect
                let _ = player.write_packet(&SoundEffect {
                    sound: Sounds::RandomClick.id(),
                    volume: 1.0,
                    pitch: 1.7,
                    pos_x: block_pos.x as f64,
                    pos_y: block_pos.y as f64,
                    pos_z: block_pos.z as f64,
                });
                
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
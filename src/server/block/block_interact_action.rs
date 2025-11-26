use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::room::secrets::DungeonSecret;
use crate::net::protocol::play::clientbound::{BlockAction, Chat, SoundEffect};
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::NoEntityImpl;
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::Player;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::redstone::is_special_lever;
use std::cell::RefCell;
use std::rc::Rc;
// use std::collections::HashMap;

use crate::server::world::ScheduledSound;
use crate::server::world::tactical_insertion::TacticalInsertionMarker;

#[derive(Debug)]
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
        secret: Rc<RefCell<DungeonSecret>>,
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
            Self::WitherDoor { door_index: id } => { //todo: left click open doors
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
                // Check if this chest is locked
                let is_locked = {
                    let dungeon = &player.server_mut().dungeon;
                    dungeon.locked_chests.get(block_pos)
                        .map(|chest_state| chest_state.locked)
                        .unwrap_or(false)
                };
                if is_locked {
                    // Chest is locked - send red message and cancel opening
                    player.write_packet(&Chat {
                        component: ChatComponentTextBuilder::new("That chest is locked!")
                            .color(MCColors::Red)
                            .build(),
                        chat_type: 0,
                    });
                    return;
                }

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
                    
                    // Increment found_secrets for the room containing this secret (only if not already obtained)
                    if let Some(room_index) = player.server_mut().dungeon.get_room_at(block_pos.x, block_pos.z) {
                        if let Some(room) = player.server_mut().dungeon.rooms.get_mut(room_index) {
                            if room.found_secrets < room.room_data.secrets {
                                room.found_secrets += 1;
                            }
                        }
                    }
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
            
            Self::MushroomBottom { set_index: _ } => {
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
                
                // Only process if not already obtained
                if !secret.obtained {
                    let world = player.world_mut();

                    world.set_block_at(Blocks::Air, block_pos.x, block_pos.y, block_pos.z);
                    world.interactable_blocks.remove(block_pos);

                    world.spawn_entity(
                        DVec3::from(block_pos).add_x(0.5).add_y(-1.4).add_z(0.5),
                        {
                            let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
                            metadata.is_invisible = true;
                            metadata
                        },
                        NoEntityImpl,
                    ).unwrap();

                    secret.obtained = true;
                    
                    // Increment found_secrets for the room containing this secret (only if not already obtained)
                    if let Some(room_index) = player.server_mut().dungeon.get_room_at(block_pos.x, block_pos.z) {
                        if let Some(room) = player.server_mut().dungeon.rooms.get_mut(room_index) {
                            if room.found_secrets < room.room_data.secrets {
                                room.found_secrets += 1;
                            }
                        }
                    }
                }
            }
            
            Self::Lever => {
                // Check if this lever unlocks any chests and unlock them
                {
                    let dungeon = &mut player.server_mut().dungeon;
                    if let Some(chest_positions) = dungeon.lever_to_chests.get(block_pos) {
                        // Unlock all chests linked to this lever
                        for chest_pos in chest_positions {
                            if let Some(chest_state) = dungeon.locked_chests.get_mut(chest_pos) {
                                if chest_state.locked {
                                    // Unlock the chest
                                    chest_state.locked = false;
                                }
                                // If already unlocked, do nothing (as per requirements)
                            }
                        }
                    }
                }
                
                let world = &mut player.server_mut().world;
                
                // Check if this is one of the special levers at the specified coordinates
                if is_special_lever(block_pos.x, block_pos.y, block_pos.z) {
                    // Toggle the lever power state
                    let current_power = world.redstone_system.get_power(*block_pos);
                    let new_power = if current_power > 0 { 0 } else { 15 };
                    world.redstone_system.set_power(*block_pos, new_power);
                    
                    // Get current lever block to toggle its state
                    let current_block = world.get_block_at(block_pos.x, block_pos.y, block_pos.z);
                    let new_lever_block = match current_block {
                        Blocks::Lever { orientation, powered } => {
                            // Toggle lever powered state
                            Blocks::Lever { 
                                orientation, 
                                powered: !powered 
                            }
                        }
                        _ => current_block, // Keep current block if not a lever
                    };
                    
                    // Update the lever block state
                    world.set_block_at(new_lever_block, block_pos.x, block_pos.y, block_pos.z);
                    
                    // Play vanilla lever click sound
                    let _ = player.write_packet(&SoundEffect {
                        sound: Sounds::RandomClick.id(),
                        volume: 0.3,
                        pitch: 0.8,
                        pos_x: block_pos.x as f64 + 0.5,
                        pos_y: block_pos.y as f64 + 0.5,
                        pos_z: block_pos.z as f64 + 0.5,
                    });
                    
                    // Send block change packet to update the lever visually
                    let _ = player.write_packet(&crate::net::protocol::play::clientbound::BlockChange {
                        block_pos: *block_pos,
                        block_state: new_lever_block.get_block_state_id(),
                    });
                    
                    // Send block action to show lever animation
                    let is_powered = match new_lever_block {
                        Blocks::Lever { powered, .. } => powered,
                        _ => false,
                    };
                    let _ = player.write_packet(&BlockAction {
                        block_pos: *block_pos,
                        event_id: 0, // Lever toggle
                        event_data: if is_powered { 1 } else { 0 }, // 1 for on, 0 for off
                        block_id: 69, // Lever block ID
                    });
                    
                    // Update redstone lanterns in the area
                    let is_powered = new_power > 0;
                    let new_lantern_block = if is_powered {
                        Blocks::LitRedstoneLamp
                    } else {
                        Blocks::RedstoneLamp
                    };
                    
                    // Check all positions around the lever for redstone lanterns
                    for x_offset in -1..=1 {
                        for y_offset in -1..=1 {
                            for z_offset in -1..=1 {
                                let check_pos = BlockPos {
                                    x: block_pos.x + x_offset,
                                    y: block_pos.y + y_offset,
                                    z: block_pos.z + z_offset,
                                };
                                
                                let current_block = world.get_block_at(check_pos.x, check_pos.y, check_pos.z);
                                if matches!(current_block, Blocks::RedstoneLamp | Blocks::LitRedstoneLamp) {
                                    // Update the lantern
                                    world.set_block_at(new_lantern_block, check_pos.x, check_pos.y, check_pos.z);
                                    
                                    // Send block change packet to update the lantern
                                    let _ = player.write_packet(&crate::net::protocol::play::clientbound::BlockChange {
                                        block_pos: check_pos,
                                        block_state: new_lantern_block.get_block_state_id(),
                                    });
                                }
                            }
                        }
                    }
                } else {
                    // Try to activate lever with falling blocks system
                    let dungeon = &mut player.server_mut().dungeon;
                    
                    // Find the room that contains this lever
                    let mut lever_found = false;
                    for room in &dungeon.rooms {
                        for lever_data in &room.lever_data {
                            let lever_pos = BlockPos {
                                x: lever_data.lever[0],
                                y: lever_data.lever[1],
                                z: lever_data.lever[2],
                            };
                            
                            if lever_pos == *block_pos {
                                // Play lever click sound
                                let _ = player.write_packet(&SoundEffect {
                                    sound: Sounds::RandomClick.id(),
                                    volume: 0.3,
                                    pitch: 0.8,
                                    pos_x: block_pos.x as f64 + 0.5,
                                    pos_y: block_pos.y as f64 + 0.5,
                                    pos_z: block_pos.z as f64 + 0.5,
                                });
                                
                                // Play anvil break sound immediately
                                let _ = player.write_packet(&SoundEffect {
                                    sound: "random.anvil_break",
                                    volume: 1.0,
                                    pitch: 1.7,
                                    pos_x: player.position.x,
                                    pos_y: player.position.y,
                                    pos_z: player.position.z,
                                });
                                
                                // Send opening message
                                player.send_message("§cYou hear the sound of something opening...");
                                
                                // Implement falling blocks animation (similar to wither doors)
                                for block_pos in &lever_data.blocks {
                                    let block_world_pos = BlockPos {
                                        x: block_pos[0],
                                        y: block_pos[1],
                                        z: block_pos[2],
                                    };
                                    
                                    // Get the current block type before replacing with barrier
                                    let current_block = world.get_block_at(block_world_pos.x, block_world_pos.y, block_world_pos.z);
                                    
                                    // Skip air blocks
                                    if matches!(current_block, Blocks::Air) {
                                        continue;
                                    }
                                    
                                    // Play RandomWoodClick sound 8 times with 250ms intervals from this block position
                                    for i in 0..8 {
                                        let delay_ticks = i * 5; // 250ms = 5 ticks at 20 TPS
                                        world.server_mut().schedule(delay_ticks, move |server| {
                                            for (_, player) in &mut server.world.players {
                                                let _ = player.write_packet(&crate::net::protocol::play::clientbound::SoundEffect {
                                                    sound: crate::server::utils::sounds::Sounds::RandomWoodClick.id(),
                                                    pos_x: block_world_pos.x as f64 + 0.5,
                                                    pos_y: block_world_pos.y as f64 + 0.5,
                                                    pos_z: block_world_pos.z as f64 + 0.5,
                                                    volume: 2.0,
                                                    pitch: 0.49,
                                                });
                                            }
                                        });
                                    }
                                    
                                    // Set barrier block to prevent passage
                                    world.set_block_at(Blocks::Barrier, block_world_pos.x, block_world_pos.y, block_world_pos.z);
                                    
                                    // Schedule the barrier block to be replaced with air after 20 ticks
                                    world.server_mut().schedule(20, move |server| {
                                        server.world.set_block_at(Blocks::Air, block_world_pos.x, block_world_pos.y, block_world_pos.z);
                                    });
                                    
                                    // Spawn falling block entity
                                    let _ = world.spawn_entity(
                                        crate::server::utils::dvec3::DVec3::new(
                                            block_world_pos.x as f64 + 0.5, 
                                            block_world_pos.y as f64 - 0.65, 
                                            block_world_pos.z as f64 + 0.5
                                        ),
                                        {
                                            let mut metadata = crate::server::entity::entity_metadata::EntityMetadata::new(crate::server::entity::entity_metadata::EntityVariant::Bat { hanging: false });
                                            metadata.is_invisible = true;
                                            metadata
                                        },
                                        crate::dungeon::room::levers::LeverEntityImpl::new(current_block, 5.0, 20),
                                    );
                                }
                                
                                // Schedule removal of barrier blocks after 20 ticks
                                let blocks_to_remove = lever_data.blocks.clone();
                                world.server_mut().schedule(20, move |server| {
                                    for block_pos in blocks_to_remove {
                                        let block_world_pos = BlockPos {
                                            x: block_pos[0],
                                            y: block_pos[1],
                                            z: block_pos[2],
                                        };
                                        server.world.set_block_at(Blocks::Air, block_world_pos.x, block_world_pos.y, block_world_pos.z);
                                    }
                                });
                                
                                // Remove the lever from interactable blocks so it can only be used once
                                world.interactable_blocks.remove(block_pos);
                                
                                lever_found = true;
                                break;
                            }
                        }
                        if lever_found {
                            break;
                        }
                    }
                    
                    if !lever_found {
                        // Lever not found in any room - play click sound and used message
                        let _ = player.write_packet(&SoundEffect {
                            sound: "random.click",
                            volume: 0.3,
                            pitch: 0.49,
                            pos_x: block_pos.x as f64,
                            pos_y: block_pos.y as f64,
                            pos_z: block_pos.z as f64,
                        });
                        
                        // Send already used message
                        player.send_message("§cThis lever has already been used.");
                    }
                }
            }
            
            Self::Callback(func) => {
                func(player, block_pos);
            }
        }
    }
}
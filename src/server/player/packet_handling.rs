use crate::net::packets::packet::ProcessPacket;
use crate::net::protocol::play::clientbound::{BlockChange, TabCompleteReply};
use crate::net::protocol::play::serverbound::*;
use crate::server::commands::Command;
use crate::server::items::Item;
use crate::server::player::container_ui::UI;
use crate::server::player::inventory::ItemSlot;
use crate::server::player::player::Player;
use std::time::{SystemTime, UNIX_EPOCH};

impl ProcessPacket for KeepAlive {
    fn process_with_player(&self, player: &mut Player) {
        if player.last_keep_alive == self.id {
            if let Ok(since) = SystemTime::now().duration_since(UNIX_EPOCH) {
                let since = since.as_millis() as i32 - player.last_keep_alive;
                player.ping = (player.ping * 3 + since) / 4;
                println!("Ping: {}", player.ping);
            }
        }
    }
}

impl ProcessPacket for ChatMessage {
    fn process_with_player(&self, player: &mut Player) {
        if self.message.starts_with("/") {
            let command = self.message.strip_prefix("/").unwrap();
            if let Err(e) = Command::handle(command, player.world_mut(), player) {
                eprintln!("cmd failed {e}")
            };
        }
    }
}

impl ProcessPacket for UseEntity {
    fn process_with_player(&self, player: &mut Player) {
        if let Some((entity, entity_impl)) = player.world_mut().entities.get_mut(&self.entity_id.0) {
            entity_impl.interact(entity, player, &self.action)
        }
    }
}

// I don't know if any implementation will be needed,
// but just in case imma keep it here
impl ProcessPacket for PlayerUpdate {}

// anti cheat stuff vvv important to do for all 3

impl ProcessPacket for PlayerPosition {
    fn process_with_player(&self, player: &mut Player) {
        player.set_position(self.x, self.y, self.z)
    }
}

impl ProcessPacket for PlayerLook {
    fn process_with_player(&self, player: &mut Player) {
        player.yaw = self.yaw;
        player.pitch = self.pitch;
    }
}

impl ProcessPacket for PlayerPositionLook {
    fn process_with_player(&self, player: &mut Player) {
        player.set_position(self.x, self.y, self.z);
        player.yaw = self.yaw;
        player.pitch = self.pitch;
    }
}

impl ProcessPacket for PlayerDigging {
    fn process_with_player(&self, player: &mut Player) {
        match self.action {
            PlayerDiggingAction::StartDestroyBlock => {
                // Check for Simon Says puzzle first
                // let action = {
                //     let world = player.world_mut();
                //     world.simon_says.handle_button_click(self.position, player.client_id, world.tick_count)
                // };
                // Simon Says puzzle handling - commented out
                // let action = {
                //     let world = player.world_mut();
                //     world.simon_says.handle_button_click(self.position, player.client_id, world.tick_count)
                // };
                // match action {
                //     Some(crate::dungeon::p3::simon_says::SimonSaysAction::BlockClick) => {
                //         return; // Block the click
                //     }
                //     Some(crate::dungeon::p3::simon_says::SimonSaysAction::ShowSolution) => {
                //         // ... (all Simon Says handling code)
                //         return;
                //     }
                //     Some(crate::dungeon::p3::simon_says::SimonSaysAction::Continue) => {
                //         return; // Continue with puzzle
                //     }
                //     Some(crate::dungeon::p3::simon_says::SimonSaysAction::SequenceCompleted) => {
                //         // ... (sequence completion handling)
                //         return;
                //     }
                //     Some(crate::dungeon::p3::simon_says::SimonSaysAction::Fail) => {
                //         return;
                //     }
                //     Some(crate::dungeon::p3::simon_says::SimonSaysAction::Completed) => {
                //         // ... (completion handling)
                //         return;
                //     }
                //     None => {
                //         // Not a Simon Says button, continue with normal processing
                //     }
                // }
                // todo:
                // when block toughness is added,
                // replace check with if vanilla toughness would match
                if let Some(ItemSlot::Filled(item, _)) = player.inventory.get_hotbar_slot(player.held_slot as usize) {
                    match item {
                        Item::DiamondPickaxe | Item::GoldenAxe => {
                            // Limit the mutable borrow of world to this scope
                            {
                                let world = player.world_mut();
                                let block = world.get_block_at(self.position.x, self.position.y, self.position.z);
                                player.write_packet(&BlockChange {
                                    block_pos: self.position,
                                    block_state: block.get_block_state_id(),
                                })
                            }
                        }
                        Item::SuperboomTNT => {
                            // Explode crypt near the targeted block
                            let yaw = player.yaw;
                            let dir = ((yaw.rem_euclid(360.0) + 45.0) / 90.0).floor() as i32 % 4; // 0=S,1=W,2=N,3=E (approx)
                            let radius = match dir {
                                0 | 3 => 3, // South or East => 3
                                _ => 2,     // North or West => 2
                            };
                            let _ = player.server_mut().dungeon.superboom_at(self.position, radius);
                        }
                        _ => {}
                    }
                }
            }
            PlayerDiggingAction::FinishDestroyBlock => {
                // Limit borrow scope
                {
                    let world = player.world_mut();
                    let block = world.get_block_at(self.position.x, self.position.y, self.position.z);
                    player.write_packet(&BlockChange {
                        block_pos: self.position,
                        block_state: block.get_block_state_id(),
                    })
                }
            }
            _ => {}
        }
    }
}

impl ProcessPacket for PlayerBlockPlacement {
    fn process_with_player(&self, player: &mut Player) {
        // Check for Simon Says puzzle first - commented out
        // if !self.position.is_invalid() {
        //     let action = {
        //         let world = player.world_mut();
        //         world.simon_says.handle_button_click(self.position, player.client_id, world.tick_count)
        //     };
        //     match action {
        // Simon Says puzzle handling - commented out
        // if !self.position.is_invalid() {
        //     let action = {
        //         let world = player.world_mut();
        //         world.simon_says.handle_button_click(self.position, player.client_id, world.tick_count)
        //     };
        //     match action {
        //         ... (all Simon Says handling code)
        //     }
        // }
        
        // Check if player is holding Bonzo Staff or Jerry-Chine Gun and handle accordingly
        if let Some(ItemSlot::Filled(item, _)) = player.inventory.get_hotbar_slot(player.held_slot as usize) {
            if let Item::BonzoStaff = item {
                // Handle Bonzo Staff block placement
                if !self.position.is_invalid() {
                    // Check if the block being clicked is interactable
                    let world = player.world_mut();
                    
                    // If it's an interactable block, don't shoot Bonzo projectile
                    if world.interactable_blocks.contains_key(&self.position) {
                        // Handle block interaction normally
                        if let Some(interact_block) = world.interactable_blocks.get(&self.position) {
                            interact_block.interact(player, &self.position);
                        }
                        return;
                    }
                }
                
                // Shoot Bonzo projectile (either air click or non-interactable block)
                if let Err(e) = player.shoot_bonzo_projectile() {
                }
                return;
            } else if let Item::JerryChineGun = item {
                // Handle Jerry-Chine Gun block placement
                if !self.position.is_invalid() {
                    // Check if the block being clicked is interactable
                    let world = player.world_mut();
                    
                    // If it's an interactable block, don't shoot Jerry projectile
                    if world.interactable_blocks.contains_key(&self.position) {
                        // Handle block interaction normally
                        if let Some(interact_block) = world.interactable_blocks.get(&self.position) {
                            interact_block.interact(player, &self.position);
                        }
                        return;
                    }
                }
                
                // Shoot Jerry projectile (either air click or non-interactable block)
                if let Err(e) = player.shoot_jerry_projectile() {
                }
                return;
            }
        }
        
        // Handle normal block placement for other items
        if !self.position.is_invalid() {
            // im considering instead of this,
            // just pass this to the dungeon, which checks doors and such

            let mut pos = self.position.clone();
            match self.placed_direction {
                0 => pos.y -= 1,
                1 => pos.y += 1,
                2 => pos.z -= 1,
                3 => pos.z += 1,
                4 => pos.x -= 1,
                _ => pos.x += 1,
            }

            {
                let world = player.world_mut();
                let block = world.get_block_at(pos.x, pos.y, pos.z);
                player.write_packet(&BlockChange {
                    block_pos: pos,
                    block_state: block.get_block_state_id()
                });
            }

            // Handle RedstoneKey right-click on redstone block (check if player has the key)
            if player.has_redstone_key {
                    let world = player.world_mut();
                    let dungeon = &mut player.server_mut().dungeon;
                    
                    // Find the room the player is in
                    if let Some(room_index) = dungeon.get_room_at(self.position.x, self.position.z) {
                        let room_name = dungeon.rooms[room_index].room_data.name.clone();
                        
                        if room_name == "Redstone Key" {
                            // Extract room data we need before we can mutate
                            let (corner, rotation, max_secrets) = {
                                let room = &dungeon.rooms[room_index];
                                (room.get_corner_pos(), room.rotation, room.room_data.secrets)
                            };
                            
                            // Redstone block is at 27/68/7 relative to room corner (needs rotation)
                            let redstone_rel_pos = crate::server::block::block_position::BlockPos { x: 27, y: 68, z: 7 }.rotate(rotation);
                            let redstone_world_pos = crate::server::block::block_position::BlockPos {
                                x: corner.x + redstone_rel_pos.x,
                                y: redstone_rel_pos.y,
                                z: corner.z + redstone_rel_pos.z,
                            };
                            
                            // Check if the clicked block is the redstone block
                            if self.position == redstone_world_pos {
                                // Check if it's actually a redstone block
                                let block = world.get_block_at(self.position.x, self.position.y, self.position.z);
                                if matches!(block, crate::server::block::blocks::Blocks::RedstoneBlock) {
                                    // Play sound: tile.piston.in, volume 1, pitch 1
                                    use crate::net::protocol::play::clientbound::SoundEffect;
                                    use crate::server::utils::sounds::Sounds;
                                    player.write_packet(&SoundEffect {
                                        sound: Sounds::PistonIn.id(),
                                        volume: 1.0,
                                        pitch: 1.0,
                                        pos_x: self.position.x as f64 + 0.5,
                                        pos_y: self.position.y as f64 + 0.5,
                                        pos_z: self.position.z as f64 + 0.5,
                                    });
                                    
                                    // Place skull on the clicked face
                                    // placed_direction: 0=bottom, 1=top, 2=north, 3=south, 4=west, 5=east
                                    let skull_direction = match self.placed_direction {
                                        0 => crate::server::utils::direction::Direction::Down,
                                        1 => crate::server::utils::direction::Direction::Up,
                                        2 => crate::server::utils::direction::Direction::North,
                                        3 => crate::server::utils::direction::Direction::South,
                                        4 => crate::server::utils::direction::Direction::West,
                                        _ => crate::server::utils::direction::Direction::East,
                                    };
                                    
                                    // Calculate position where skull should be placed (adjacent to clicked face)
                                    let mut skull_pos = self.position.clone();
                                    match self.placed_direction {
                                        0 => skull_pos.y -= 1, // bottom
                                        1 => skull_pos.y += 1, // top
                                        2 => skull_pos.z -= 1, // north
                                        3 => skull_pos.z += 1, // south
                                        4 => skull_pos.x -= 1, // west
                                        _ => skull_pos.x += 1, // east
                                    }
                                    
                                    // Place the skull block
                                    world.set_block_at(
                                        crate::server::block::blocks::Blocks::Skull { 
                                            direction: skull_direction, 
                                            no_drop: false 
                                        },
                                        skull_pos.x,
                                        skull_pos.y,
                                        skull_pos.z
                                    );
                                    
                                    // Send UpdateBlockEntity with skull NBT
                                    use crate::net::protocol::play::clientbound::UpdateBlockEntity;
                                    use crate::server::utils::nbt::serialize::serialize_nbt;
                                    use crate::dungeon::room::secrets::DungeonSecret;
                                    let skull_owner = DungeonSecret::create_redstone_key_skull_nbt();
                                    let full_te_nbt = crate::server::utils::nbt::nbt::NBT::with_nodes(vec![
                                        crate::server::utils::nbt::nbt::NBT::string("id", "Skull"),
                                        crate::server::utils::nbt::nbt::NBT::int("x", skull_pos.x),
                                        crate::server::utils::nbt::nbt::NBT::int("y", skull_pos.y),
                                        crate::server::utils::nbt::nbt::NBT::int("z", skull_pos.z),
                                        crate::server::utils::nbt::nbt::NBT::byte("SkullType", 3), // 3 = player head
                                        skull_owner,
                                    ]);
                                    let nbt_bytes = serialize_nbt(&full_te_nbt);
                                    let update_packet = UpdateBlockEntity {
                                        block_pos: skull_pos,
                                        action: 4, // 4 = skull update in 1.8
                                        nbt_data: Some(nbt_bytes.clone()),
                                    };
                                    for (_, other_player) in &mut world.players {
                                        other_player.write_packet(&update_packet);
                                    }
                                    let chunk_x = skull_pos.x >> 4;
                                    let chunk_z = skull_pos.z >> 4;
                                    if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                                        chunk.packet_buffer.write_packet(&update_packet);
                                    }
                                    
                                    // Make blocks at 18/68/23, 19/68/23, 20/68/23, 21/68/23 disappear
                                    let blocks_to_remove = [
                                        (18, 68, 23),
                                        (19, 68, 23),
                                        (20, 68, 23),
                                        (21, 68, 23),
                                    ];
                                    
                                    for (rel_x, rel_y, rel_z) in &blocks_to_remove {
                                        let block_rel_pos = crate::server::block::block_position::BlockPos { 
                                            x: *rel_x, 
                                            y: *rel_y, 
                                            z: *rel_z 
                                        }.rotate(rotation);
                                        let block_world_pos = crate::server::block::block_position::BlockPos {
                                            x: corner.x + block_rel_pos.x,
                                            y: *rel_y,
                                            z: corner.z + block_rel_pos.z,
                                        };
                                        
                                        // Remove the block
                                        world.set_block_at(
                                            crate::server::block::blocks::Blocks::Air,
                                            block_world_pos.x,
                                            block_world_pos.y,
                                            block_world_pos.z
                                        );
                                    }
                                    
                                    // Spawn secret chest at 28/62/23 facing west (needs rotation)
                                    let chest_rel_pos = crate::server::block::block_position::BlockPos { x: 28, y: 62, z: 23 }.rotate(rotation);
                                    let chest_world_pos = crate::server::block::block_position::BlockPos {
                                        x: corner.x + chest_rel_pos.x,
                                        y: chest_rel_pos.y,
                                        z: corner.z + chest_rel_pos.z,
                                    };
                                    
                                    // Rotate the chest direction based on room rotation
                                    use crate::server::block::rotatable::Rotatable;
                                    let chest_direction = crate::server::utils::direction::Direction::West.rotate(rotation);
                                    
                                    world.set_block_at(
                                        crate::server::block::blocks::Blocks::Chest { 
                                            direction: chest_direction
                                        },
                                        chest_world_pos.x,
                                        chest_world_pos.y,
                                        chest_world_pos.z,
                                    );
                                    
                                    // Create a secret for the chest
                                    use std::rc::Rc;
                                    use std::cell::RefCell;
                                    use crate::dungeon::room::secrets::SecretType;
                                    let secret = Rc::new(RefCell::new(DungeonSecret::new(
                                        SecretType::SecretChest { 
                                            direction: chest_direction
                                        },
                                        chest_world_pos,
                                        8.0, // spawn radius
                                    )));
                                    secret.borrow_mut().has_spawned = true;
                                    secret.borrow_mut().obtained = false;
                                    
                                    // Register chest as interactable
                                    world.interactable_blocks.insert(chest_world_pos, crate::server::block::block_interact_action::BlockInteractAction::Chest {
                                        secret: secret.clone(),
                                    });
                                    
                                    // Count this as a secret for the room
                                    // Use the room_index we already have (chest is in the same room)
                                    let should_update_map = {
                                        let room = dungeon.rooms.get(room_index);
                                        room.map(|r| r.found_secrets < max_secrets && r.entered).unwrap_or(false)
                                    };
                                    
                                    if let Some(room) = dungeon.rooms.get_mut(room_index) {
                                        if room.found_secrets < max_secrets {
                                            room.found_secrets += 1;
                                        }
                                    }
                                    
                                    // Update map if room is entered and secret count changed
                                    if should_update_map {
                                        dungeon.update_map_for_room(room_index);
                                    }
                                    
                                    // Remove RedstoneKey from player (they used it)
                                    player.has_redstone_key = false;
                                    
                                    // Remove the original redstone key skull interactable block so it can't be picked up again
                                    let (rel_x, rel_y, rel_z) = match rotation {
                                        crate::server::utils::direction::Direction::North | crate::server::utils::direction::Direction::South => (19, 66, 7),
                                        crate::server::utils::direction::Direction::East | crate::server::utils::direction::Direction::West => (10, 70, 26),
                                        _ => (19, 66, 7),
                                    };
                                    let original_skull_pos = crate::server::block::block_position::BlockPos { x: rel_x, y: rel_y, z: rel_z }.rotate(rotation);
                                    let original_skull_world_pos = crate::server::block::block_position::BlockPos {
                                        x: corner.x + original_skull_pos.x,
                                        y: original_skull_pos.y,
                                        z: corner.z + original_skull_pos.z,
                                    };
                                    world.interactable_blocks.remove(&original_skull_world_pos);
                                    
                                    // Send message: &r&7&oYou hear something open...&r
                                    use crate::server::player::dungeon_stats::legacy_to_chat_component;
                                    use crate::net::protocol::play::clientbound::Chat;
                                    player.write_packet(&Chat {
                                        component: legacy_to_chat_component("&r&7&oYou hear something open...&r"),
                                        chat_type: 0,
                                    });
                                    
                                    return; // Don't process further
                                }
                            }
                        } else if room_name == "Golden Oasis" {
                            // Extract room data we need before we can mutate
                            let (corner, max_secrets) = {
                                let room = &dungeon.rooms[room_index];
                                (room.get_corner_pos(), room.room_data.secrets)
                            };
                            
                            // Redstone block is at 10/71/3 relative to room corner (no rotation needed)
                            let redstone_rel_pos = crate::server::block::block_position::BlockPos { x: 10, y: 71, z: 3 };
                            let redstone_world_pos = crate::server::block::block_position::BlockPos {
                                x: corner.x + redstone_rel_pos.x,
                                y: redstone_rel_pos.y,
                                z: corner.z + redstone_rel_pos.z,
                            };
                            
                            // Check if the clicked block is the redstone block
                            if self.position == redstone_world_pos {
                                // Check if it's actually a redstone block
                                let block = world.get_block_at(self.position.x, self.position.y, self.position.z);
                                if matches!(block, crate::server::block::blocks::Blocks::RedstoneBlock) {
                                    // Play sound: tile.piston.in, volume 1, pitch 1
                                    use crate::net::protocol::play::clientbound::SoundEffect;
                                    use crate::server::utils::sounds::Sounds;
                                    player.write_packet(&SoundEffect {
                                        sound: Sounds::PistonIn.id(),
                                        volume: 1.0,
                                        pitch: 1.0,
                                        pos_x: self.position.x as f64 + 0.5,
                                        pos_y: self.position.y as f64 + 0.5,
                                        pos_z: self.position.z as f64 + 0.5,
                                    });
                                    
                                    // Place skull on the clicked face
                                    let skull_direction = match self.placed_direction {
                                        0 => crate::server::utils::direction::Direction::Down,
                                        1 => crate::server::utils::direction::Direction::Up,
                                        2 => crate::server::utils::direction::Direction::North,
                                        3 => crate::server::utils::direction::Direction::South,
                                        4 => crate::server::utils::direction::Direction::West,
                                        _ => crate::server::utils::direction::Direction::East,
                                    };
                                    
                                    // Calculate position where skull should be placed (adjacent to clicked face)
                                    let mut skull_pos = self.position.clone();
                                    match self.placed_direction {
                                        0 => skull_pos.y -= 1, // bottom
                                        1 => skull_pos.y += 1, // top
                                        2 => skull_pos.z -= 1, // north
                                        3 => skull_pos.z += 1, // south
                                        4 => skull_pos.x -= 1, // west
                                        _ => skull_pos.x += 1, // east
                                    }
                                    
                                    // Place the skull block
                                    world.set_block_at(
                                        crate::server::block::blocks::Blocks::Skull { 
                                            direction: skull_direction, 
                                            no_drop: false 
                                        },
                                        skull_pos.x,
                                        skull_pos.y,
                                        skull_pos.z
                                    );
                                    
                                    // Send UpdateBlockEntity with skull NBT
                                    use crate::net::protocol::play::clientbound::UpdateBlockEntity;
                                    use crate::server::utils::nbt::serialize::serialize_nbt;
                                    use crate::dungeon::room::secrets::DungeonSecret;
                                    let skull_owner = DungeonSecret::create_redstone_key_skull_nbt();
                                    let full_te_nbt = crate::server::utils::nbt::nbt::NBT::with_nodes(vec![
                                        crate::server::utils::nbt::nbt::NBT::string("id", "Skull"),
                                        crate::server::utils::nbt::nbt::NBT::int("x", skull_pos.x),
                                        crate::server::utils::nbt::nbt::NBT::int("y", skull_pos.y),
                                        crate::server::utils::nbt::nbt::NBT::int("z", skull_pos.z),
                                        crate::server::utils::nbt::nbt::NBT::byte("SkullType", 3), // 3 = player head
                                        skull_owner,
                                    ]);
                                    let nbt_bytes = serialize_nbt(&full_te_nbt);
                                    let update_packet = UpdateBlockEntity {
                                        block_pos: skull_pos,
                                        action: 4, // 4 = skull update in 1.8
                                        nbt_data: Some(nbt_bytes.clone()),
                                    };
                                    for (_, other_player) in &mut world.players {
                                        other_player.write_packet(&update_packet);
                                    }
                                    let chunk_x = skull_pos.x >> 4;
                                    let chunk_z = skull_pos.z >> 4;
                                    if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                                        chunk.packet_buffer.write_packet(&update_packet);
                                    }
                                    
                                    // Make blocks at 26/70/25, 27/70/25, 28/70/25 disappear (no rotation)
                                    let blocks_to_remove = [
                                        (26, 70, 25),
                                        (27, 70, 25),
                                        (28, 70, 25),
                                    ];
                                    
                                    for (rel_x, rel_y, rel_z) in &blocks_to_remove {
                                        let block_world_pos = crate::server::block::block_position::BlockPos {
                                            x: corner.x + *rel_x,
                                            y: *rel_y,
                                            z: corner.z + *rel_z,
                                        };
                                        
                                        // Remove the block
                                        world.set_block_at(
                                            crate::server::block::blocks::Blocks::Air,
                                            block_world_pos.x,
                                            block_world_pos.y,
                                            block_world_pos.z
                                        );
                                    }
                                    
                                    // Spawn two secret chests: 13/63/25 and 12/63/23, both facing east (no rotation)
                                    let chests_to_spawn = [
                                        (13, 63, 25),
                                        (12, 63, 23),
                                    ];
                                    
                                    for (rel_x, rel_y, rel_z) in &chests_to_spawn {
                                        let chest_world_pos = crate::server::block::block_position::BlockPos {
                                            x: corner.x + *rel_x,
                                            y: *rel_y,
                                            z: corner.z + *rel_z,
                                        };
                                        
                                        world.set_block_at(
                                            crate::server::block::blocks::Blocks::Chest { 
                                                direction: crate::server::utils::direction::Direction::East
                                            },
                                            chest_world_pos.x,
                                            chest_world_pos.y,
                                            chest_world_pos.z,
                                        );
                                        
                                        // Create a secret for each chest
                                        use std::rc::Rc;
                                        use std::cell::RefCell;
                                        use crate::dungeon::room::secrets::{DungeonSecret, SecretType};
                                        let secret = Rc::new(RefCell::new(DungeonSecret::new(
                                            SecretType::SecretChest { 
                                                direction: crate::server::utils::direction::Direction::East
                                            },
                                            chest_world_pos,
                                            8.0, // spawn radius
                                        )));
                                        secret.borrow_mut().has_spawned = true;
                                        secret.borrow_mut().obtained = false;
                                        
                                        // Register chest as interactable
                                        world.interactable_blocks.insert(chest_world_pos, crate::server::block::block_interact_action::BlockInteractAction::Chest {
                                            secret: secret.clone(),
                                        });
                                        
                                        // Count this as a secret for the room
                                        if let Some(room_index) = dungeon.get_room_at(chest_world_pos.x, chest_world_pos.z) {
                                            let should_update_map = {
                                                let room = dungeon.rooms.get(room_index);
                                                room.map(|r| r.found_secrets < r.room_data.secrets && r.entered).unwrap_or(false)
                                            };
                                            
                                            if let Some(room) = dungeon.rooms.get_mut(room_index) {
                                                if room.found_secrets < room.room_data.secrets {
                                                    room.found_secrets += 1;
                                                }
                                            }
                                            
                                            // Update map if room is entered and secret count changed
                                            if should_update_map {
                                                dungeon.update_map_for_room(room_index);
                                            }
                                        }
                                    }
                                    
                                    // Remove RedstoneKey from player (they used it)
                                    player.has_redstone_key = false;
                                    
                                    // Remove the original redstone key skull interactable block so it can't be picked up again
                                    let original_skull_rel_pos = crate::server::block::block_position::BlockPos { x: 12, y: 71, z: 8 };
                                    let original_skull_world_pos = crate::server::block::block_position::BlockPos {
                                        x: corner.x + original_skull_rel_pos.x,
                                        y: original_skull_rel_pos.y,
                                        z: corner.z + original_skull_rel_pos.z,
                                    };
                                    world.interactable_blocks.remove(&original_skull_world_pos);
                                    
                                    // Send message: &r&7&oYou hear something open...&r
                                    use crate::server::player::dungeon_stats::legacy_to_chat_component;
                                    use crate::net::protocol::play::clientbound::Chat;
                                    player.write_packet(&Chat {
                                        component: legacy_to_chat_component("&r&7&oYou hear something open...&r"),
                                        chat_type: 0,
                                    });
                                    
                                    return; // Don't process further
                                }
                            }
                        } else if room_name == "Redstone Crypt" {
                            // Extract room data we need before we can mutate
                            let (corner, room_rotation, max_secrets) = {
                                let room = &dungeon.rooms[room_index];
                                (room.get_corner_pos(), room.rotation, room.room_data.secrets)
                            };
                            
                            // Redstone block is at 26/72/6 relative to room corner (rotated based on room rotation)
                            let redstone_rel_pos = crate::server::block::block_position::BlockPos { x: 26, y: 72, z: 6 }.rotate(room_rotation);
                            let redstone_world_pos = crate::server::block::block_position::BlockPos {
                                x: corner.x + redstone_rel_pos.x,
                                y: redstone_rel_pos.y,
                                z: corner.z + redstone_rel_pos.z,
                            };
                            
                            // Check if the clicked block is the redstone block
                            if self.position == redstone_world_pos {
                                // Check if it's actually a redstone block
                                let block = world.get_block_at(self.position.x, self.position.y, self.position.z);
                                if matches!(block, crate::server::block::blocks::Blocks::RedstoneBlock) {
                                    // Play sound: tile.piston.in, volume 1, pitch 1
                                    use crate::net::protocol::play::clientbound::SoundEffect;
                                    use crate::server::utils::sounds::Sounds;
                                    player.write_packet(&SoundEffect {
                                        sound: Sounds::PistonIn.id(),
                                        volume: 1.0,
                                        pitch: 1.0,
                                        pos_x: self.position.x as f64 + 0.5,
                                        pos_y: self.position.y as f64 + 0.5,
                                        pos_z: self.position.z as f64 + 0.5,
                                    });
                                    
                                    // Place skull on the clicked face
                                    // placed_direction: 0=bottom, 1=top, 2=north, 3=south, 4=west, 5=east
                                    let skull_direction = match self.placed_direction {
                                        0 => crate::server::utils::direction::Direction::Down,
                                        1 => crate::server::utils::direction::Direction::Up,
                                        2 => crate::server::utils::direction::Direction::North,
                                        3 => crate::server::utils::direction::Direction::South,
                                        4 => crate::server::utils::direction::Direction::West,
                                        _ => crate::server::utils::direction::Direction::East,
                                    };
                                    
                                    // Calculate position where skull should be placed (adjacent to clicked face)
                                    let mut skull_pos = self.position.clone();
                                    match self.placed_direction {
                                        0 => skull_pos.y -= 1, // bottom
                                        1 => skull_pos.y += 1, // top
                                        2 => skull_pos.z -= 1, // north
                                        3 => skull_pos.z += 1, // south
                                        4 => skull_pos.x -= 1, // west
                                        _ => skull_pos.x += 1, // east
                                    }
                                    
                                    // Place the skull block
                                    world.set_block_at(
                                        crate::server::block::blocks::Blocks::Skull { 
                                            direction: skull_direction, 
                                            no_drop: false 
                                        },
                                        skull_pos.x,
                                        skull_pos.y,
                                        skull_pos.z
                                    );
                                    
                                    // Send UpdateBlockEntity with skull NBT
                                    use crate::net::protocol::play::clientbound::UpdateBlockEntity;
                                    use crate::server::utils::nbt::serialize::serialize_nbt;
                                    use crate::dungeon::room::secrets::DungeonSecret;
                                    let skull_owner = DungeonSecret::create_redstone_key_skull_nbt();
                                    let full_te_nbt = crate::server::utils::nbt::nbt::NBT::with_nodes(vec![
                                        crate::server::utils::nbt::nbt::NBT::string("id", "Skull"),
                                        crate::server::utils::nbt::nbt::NBT::int("x", skull_pos.x),
                                        crate::server::utils::nbt::nbt::NBT::int("y", skull_pos.y),
                                        crate::server::utils::nbt::nbt::NBT::int("z", skull_pos.z),
                                        crate::server::utils::nbt::nbt::NBT::byte("SkullType", 3), // 3 = player head
                                        skull_owner,
                                    ]);
                                    let nbt_bytes = serialize_nbt(&full_te_nbt);
                                    let update_packet = UpdateBlockEntity {
                                        block_pos: skull_pos,
                                        action: 4, // 4 = skull update in 1.8
                                        nbt_data: Some(nbt_bytes.clone()),
                                    };
                                    for (_, other_player) in &mut world.players {
                                        other_player.write_packet(&update_packet);
                                    }
                                    let chunk_x = skull_pos.x >> 4;
                                    let chunk_z = skull_pos.z >> 4;
                                    if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
                                        chunk.packet_buffer.write_packet(&update_packet);
                                    }
                                    
                                    // Register as interactable block
                                    world.interactable_blocks.insert(skull_pos, crate::server::block::block_interact_action::BlockInteractAction::RedstoneKeySkull {
                                        room_index: room_index,
                                    });
                                    
                                    // Make blocks at 12/70/15, 13/70/15, 14/70/15, 15/70/15, 13/69/15, 14/69/15, 15/69/15 disappear (rotated based on room rotation)
                                    let blocks_to_remove = [
                                        (12, 70, 15),
                                        (13, 70, 15),
                                        (14, 70, 15),
                                        (15, 70, 15),
                                        (13, 69, 15),
                                        (14, 69, 15),
                                        (15, 69, 15),
                                    ];
                                    
                                    for (rel_x, rel_y, rel_z) in &blocks_to_remove {
                                        let block_rel_pos = crate::server::block::block_position::BlockPos { x: *rel_x, y: *rel_y, z: *rel_z }.rotate(room_rotation);
                                        let block_world_pos = crate::server::block::block_position::BlockPos {
                                            x: corner.x + block_rel_pos.x,
                                            y: block_rel_pos.y,
                                            z: corner.z + block_rel_pos.z,
                                        };
                                        
                                        // Remove the block
                                        world.set_block_at(
                                            crate::server::block::blocks::Blocks::Air,
                                            block_world_pos.x,
                                            block_world_pos.y,
                                            block_world_pos.z
                                        );
                                    }
                                    
                                    // Remove RedstoneKey from player (they used it)
                                    player.has_redstone_key = false;
                                    
                                    // Remove the original redstone key skull interactable block so it can't be picked up again
                                    let original_skull_rel_pos = crate::server::block::block_position::BlockPos { x: 4, y: 71, z: 4 }.rotate(room_rotation);
                                    let original_skull_world_pos = crate::server::block::block_position::BlockPos {
                                        x: corner.x + original_skull_rel_pos.x,
                                        y: original_skull_rel_pos.y,
                                        z: corner.z + original_skull_rel_pos.z,
                                    };
                                    world.interactable_blocks.remove(&original_skull_world_pos);
                                    
                                    // Count this as a secret for the room
                                    let should_update_map = {
                                        let room = dungeon.rooms.get(room_index);
                                        room.map(|r| r.found_secrets < max_secrets && r.entered).unwrap_or(false)
                                    };
                                    
                                    if let Some(room) = dungeon.rooms.get_mut(room_index) {
                                        if room.found_secrets < max_secrets {
                                            room.found_secrets += 1;
                                        }
                                    }
                                    
                                    // Update map if room is entered and secret count changed
                                    if should_update_map {
                                        dungeon.update_map_for_room(room_index);
                                    }
                                    
                                    // Send message: &r&7&oYou hear something open...&r
                                    use crate::server::player::dungeon_stats::legacy_to_chat_component;
                                    use crate::net::protocol::play::clientbound::Chat;
                                    player.write_packet(&Chat {
                                        component: legacy_to_chat_component("&r&7&oYou hear something open...&r"),
                                        chat_type: 0,
                                    });
                                    
                                    return; // Don't process further
                                }
                            }
                        }
                    }
                }
            
            // Use Superboom TNT when right-clicking a block
            if let Some(ItemSlot::Filled(item, _)) = player.inventory.get_hotbar_slot(player.held_slot as usize) {
                if let Item::SuperboomTNT = item {
                    let yaw = player.yaw;
                    let dir = ((yaw.rem_euclid(360.0) + 45.0) / 90.0).floor() as i32 % 4; // 0=S,1=W,2=N,3=E (approx)
                    let radius = match dir {
                        0 | 3 => 3, // South or East => 3
                        _ => 2,     // North or West => 2
                    };
                    let _ = player.server_mut().dungeon.superboom_at(pos, radius);
                }
            }

            {
                let world = player.world_mut();
                if let Some(interact_block) = world.interactable_blocks.get(&self.position) {
                    interact_block.interact(player, &self.position);
                }
            }
        } else {
            player.handle_right_click();
        }
        // player.sync_inventory();
    }
}

impl ProcessPacket for HeldItemChange {
    fn process_with_player(&self, player: &mut Player) {
        // warn player if invalid packets
        let item_slot = self.slot_id.clamp(0, 8) as u8;
        player.held_slot = item_slot;
    }
}

// will be useful if we want to add stuff like mage beam
impl ProcessPacket for ArmSwing {}

impl ProcessPacket for PlayerAction {
    fn process_with_player(&self, player: &mut Player) {
        match self.action {
            PlayerActionType::StartSneaking => player.is_sneaking = true,
            PlayerActionType::StopSneaking => player.is_sneaking = false,
            _ => {}
        }
    }
}

impl ProcessPacket for CloseWindow {
    fn process_with_player(&self, player: &mut Player) {
        player.open_ui(UI::None)
    }
}

impl ProcessPacket for ClickWindow {
    fn process_with_player(&self, player: &mut Player) {
        if player.current_ui == UI::None
            || (player.window_id != self.window_id && player.current_ui != UI::Inventory)
        {
            player.sync_inventory();
            return;
        }
        player.current_ui.clone().handle_click_window(self, player);
    }
}

impl ProcessPacket for ConfirmTransaction {
    // wd sync stuff
}

impl ProcessPacket for TabComplete {
    fn process_with_player(&self, player: &mut Player) {
        if !self.message.starts_with("/") {
            return;
        }
        let parts: Vec<&str> = self.message.split_whitespace().collect();
        let command_name = parts[0].strip_prefix("/").unwrap();

        if command_name.is_empty() {
            player.write_packet(&TabCompleteReply {
                matches: Command::list().iter().map(|cmd| format!("/{}", cmd.name())).collect(),
            });
            return
        }

        if let Some(command) = Command::find(command_name) {
            let args = &parts[1..];

            let next_arg = self.message.ends_with(' ');

            if args.is_empty() && !next_arg {
                // user input a valid command but has not hit space, so we shouldn't provide any completions.
                // there might be a better way to do this somewhere else but idk atm.
                return;
            }

            let current_arg = if next_arg {
                args.len()
            } else {
                args.len().saturating_sub(1)
            };

            let command_args = command.args(player.world_mut(), player);

            if current_arg >= command_args.len() {
                // user has input too many arguments; so we just return here.
                return;
            }

            let completions = {
                let arg = &command_args.get(current_arg);
                if arg.is_none() { return; }
                &arg.unwrap().completions
            };

            let matches: Vec<String> = if next_arg || args.is_empty() {
                completions.to_vec()
            } else {
                completions.iter().filter(|cmp| cmp.starts_with(args.last().unwrap_or(&""))).cloned().collect()
            };

            player.write_packet(&TabCompleteReply {
                matches
            });
        } else {
            let commands = Command::list().iter().filter(|cmd| cmd.name().starts_with(command_name)).map(|cmd| format!("/{}", cmd.name())).collect();
            player.write_packet(&TabCompleteReply {
                matches: commands
            });
        }
    }
}

impl ProcessPacket for ClientSettings {
    // render distance stuff
}

impl ProcessPacket for ClientStatus {
    fn process_with_player(&self, player: &mut Player) {
        match self {
            ClientStatus::OpenInventory => {
                player.open_ui(UI::Inventory)
            }
            _ => {}
        }
    }
}

impl ProcessPacket for CustomPayload {
    fn process_with_player(&self, _player: &mut Player) {
        // Log the received plugin message but don't process it
        // Never disconnect on plugin messages - vanilla, Forge, Essential, Skytils, etc. all send them
        
        match self.channel.as_str() {
            // Client brand handshake - safe to ignore
            "MC|Brand" => {
                // Payload is a MC string containing the client brand, we can ignore it
            }
            // Client registers channels it wants to receive - safe to ignore
            "REGISTER" => {
                // Payload is a list of channel names separated by \0, safe to ignore
            }
            // Forge/FML channels - safe to ignore
            "FML|HS" | "FML" | "FML|MP" => {
                // Forge mod loader handshake - safe to ignore
            }
            // Everything else - just ignore
            _ => {
                // Unknown channels are fine - just ignore them
            }
        }
        
        let hex_dump: String = self.payload.iter()
            .take(32) // Show first 32 bytes
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(" ");
        let hex_suffix = if self.payload.len() > 32 { "..." } else { "" };
        
        println!(
            "[RC DEBUG] received plugin message from client: channel='{}', len={}, hex={}{}",
            self.channel,
            self.payload.len(),
            hex_dump,
            hex_suffix
        );
        // IMPORTANT: Return Ok (implicit) - never disconnect on plugin messages
    }
}
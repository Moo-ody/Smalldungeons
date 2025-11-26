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
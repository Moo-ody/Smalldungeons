use crate::net::packets::packet::ProcessPacket;
use crate::net::protocol::play::clientbound::{BlockChange, TabCompleteReply};
use crate::net::protocol::play::serverbound::{ArmSwing, ChatMessage, ClickWindow, ClientSettings, ClientStatus, CloseWindow, ConfirmTransaction, HeldItemChange, KeepAlive, PlayerAction, PlayerActionType, PlayerBlockPlacement, PlayerDigging, PlayerDiggingAction, PlayerLook, PlayerPosition, PlayerPositionLook, PlayerUpdate, TabComplete};
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
        let world = player.world_mut();
        match self.action {
            PlayerDiggingAction::StartDestroyBlock => {
                // todo:
                // when block toughness is added,
                // replace check with if vanilla toughness would match
                if matches!(player.inventory.get_hotbar_slot(player.held_slot as usize), Some(ItemSlot::Filled(Item::DiamondPickaxe))) {
                    let block = world.get_block_at(self.position.x, self.position.y, self.position.z);
                    player.write_packet(&BlockChange {
                        block_pos: self.position,
                        block_state: block.get_block_state_id(),
                    })
                }
            }
            PlayerDiggingAction::FinishDestroyBlock => {
                let block = world.get_block_at(self.position.x, self.position.y, self.position.z);
                player.write_packet(&BlockChange {
                    block_pos: self.position,
                    block_state: block.get_block_state_id(),
                })
            }
            _ => {}
        }
    }
}

impl ProcessPacket for PlayerBlockPlacement {
    fn process_with_player(&self, player: &mut Player) {
        // todo:
        // - improve accuracy
        // - prevent clicking from a distance
        let world = player.world_mut();
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

            let block = world.get_block_at(pos.x, pos.y, pos.z);
            player.write_packet(&BlockChange {
                block_pos: pos,
                block_state: block.get_block_state_id()
            });

            if let Some(interact_block) = world.interactable_blocks.get(&self.position) {
                interact_block.interact(player, &self.position);
                return;
            }
        }
        player.handle_right_click();
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
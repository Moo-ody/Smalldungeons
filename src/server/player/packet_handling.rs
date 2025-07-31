use crate::net::packets::packet::ProcessPacket;
use crate::net::protocol::play::clientbound::BlockChange;
use crate::net::protocol::play::serverbound::{ArmSwing, ChatMessage, ClickWindow, ClientSettings, ClientStatus, CloseWindow, ConfirmTransaction, HeldItemChange, KeepAlive, PlayerAction, PlayerActionType, PlayerBlockPlacement, PlayerDigging, PlayerDiggingAction, PlayerLook, PlayerPosition, PlayerPositionLook, PlayerUpdate};
use crate::server::items::Item;
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
        if self.message.as_str() == "/locraw" {
            player.send_message(r#"{"server":"mini237V","gametype":"SKYBLOCK","mode":"dungeon","map":"Dungeon"}"#);
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
                if let Some(ItemSlot::Filled(Item::DiamondPickaxe)) = player.inventory.get_hotbar_slot(player.held_slot as usize) {
                    let block = world.get_block_at(self.position.x, self.position.y, self.position.z);
                    player.write_packet(&BlockChange {
                        block_pos: *&self.position,
                        block_state: block.get_block_state_id(),
                    })
                }
            }
            PlayerDiggingAction::FinishDestroyBlock => {
                let block = world.get_block_at(self.position.x, self.position.y, self.position.z);
                player.write_packet(&BlockChange {
                    block_pos: *&self.position,
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


// todo: re-implement gui stuff
impl ProcessPacket for CloseWindow {
    fn process_with_player(&self, player: &mut Player) {
    }
}

impl ProcessPacket for ClickWindow {
    fn process_with_player(&self, player: &mut Player) {
        println!("item stack: {:?}", self.clicked_item);
        player.inventory.click_slot(self, &mut player.packet_buffer);
        player.sync_inventory()
    }
}

impl ProcessPacket for ConfirmTransaction {
    // wd sync stuff
}

impl ProcessPacket for ClientSettings {
    // render distance stuff
}

impl ProcessPacket for ClientStatus {
    fn process_with_player(&self, player: &mut Player) {
        // todo gui stuff
        match self {
            ClientStatus::OpenInventory => {}
            _ => {}
        }
    }
}
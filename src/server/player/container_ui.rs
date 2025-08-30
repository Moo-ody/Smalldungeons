use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::dungeon_state::DungeonState::NotReady;
use crate::net::protocol::play::clientbound::CloseWindow;
use crate::net::protocol::play::serverbound::ClickWindow;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::{ClientId, Player};
use crate::server::server::Server;
use crate::server::utils::nbt::nbt::NBT;
use crate::server::utils::sounds::Sounds;
use crate::net::protocol::play::clientbound::SoundEffect;

#[derive(Debug)]
pub struct ContainerData {
    pub title: String,
    pub slot_amount: u8,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum UI {
    None,
    // this is here to direct clicks to the actual inventory where all the items are stored, etc.
    Inventory,
    MortReadyUpMenu
}

impl UI {

    /// this function returns data for opening a container,
    /// should not be used for UI's that don't use a container
    pub fn get_container_data(&self) -> Option<ContainerData> {
        match self {
            UI::MortReadyUpMenu => Some(ContainerData {
                title: "Ready Up".to_string(),
                slot_amount: 54,
            }),
            _ => None
        }
    }

    /// returns a list of items to send to client 
    pub fn get_container_contents(&self, server: &Server, client_id: &ClientId) -> Option<Vec<Option<ItemStack>>> {
        let player = server.world.players.get(client_id)?;
        match self {
            UI::MortReadyUpMenu => {
                let mut content = default_container_content(54);

                let (item_name, color) = if let NotReady = server.dungeon.state {
                    ("§cNot Ready", 14)
                } else {
                    ("§aReady", 13)
                };
                content[4] = Some(ItemStack {
                    item: 397,
                    stack_size: 1,
                    metadata: 3,
                    tag_compound: Some(NBT::with_nodes(vec![
                        NBT::compound("display", vec![
                            NBT::string("Name", &format!("§7{}", player.profile.username)),
                            NBT::list_from_string("Lore", &item_name.to_string())
                        ]),
                        NBT::string("SkullOwner", &player.profile.username),
                    ])),
                });
                content[13] = Some(ItemStack {
                    item: 95,
                    stack_size: 1,
                    metadata: color,
                    tag_compound: Some(NBT::with_nodes(vec![
                        NBT::compound("display", vec![
                            NBT::string("Name", item_name)
                        ])
                    ])),
                });
                content[49] = Some(ItemStack {
                    item: 166,
                    stack_size: 1,
                    metadata: 0,
                    tag_compound: Some(NBT::with_nodes(vec![
                        NBT::compound("display", vec![
                            NBT::string("Name", "§cClose")
                        ])
                    ])),
                });
                Some(content)
            }
            _ => None
        }
    }
    
    /// handles the click window packet for all UI
    pub fn handle_click_window(
        &self,
        packet: &ClickWindow,
        player: &mut Player,
    ) {
        match self {
            UI::Inventory => {
                if packet.slot_id == 44 {
                    player.sync_inventory();
                    return;
                }
                if player.inventory.click_slot(packet, &mut player.packet_buffer) {
                    player.sync_inventory();
                }
            },
            UI::MortReadyUpMenu => {
                match packet.slot_id {
                    4 | 13 => {
                        let dung = &mut player.server_mut().dungeon;
                        match dung.state {
                            NotReady => {
                                // Send "is now ready!" message to all players
                                let ready_msg = format!("§a{} is now ready!", player.profile.username);
                                for (_, other_player) in &mut player.server_mut().world.players {
                                    let _ = other_player.send_message(&ready_msg);
                                }
                                
                                // Play first random.click sound when ready
                                for (_, other_player) in &mut player.server_mut().world.players {
                                    let _ = other_player.write_packet(&SoundEffect {
                                        sound: Sounds::RandomClick.id(),
                                        volume: 0.55,
                                        pitch: 2.0,
                                        pos_x: other_player.position.x,
                                        pos_y: other_player.position.y,
                                        pos_z: other_player.position.z,
                                    });
                                }
                                
                                // Start the dungeon countdown
                                dung.state = DungeonState::Starting { tick_countdown: 100 };
                            }
                            DungeonState::Starting { .. } => dung.state = NotReady,
                            _ => {}
                        }
                    }
                    49 => {
                        player.current_ui = UI::None;
                        player.write_packet(&CloseWindow {
                            window_id: player.window_id,
                        });
                    },
                    _ => {}
                }
                player.sync_inventory();
            }
            _ => unreachable!()
        }
    }
}

/// returns a vec with size contained only black stained-glass panes with no name.
/// used as a background for a container
fn default_container_content(size: usize) -> Vec<Option<ItemStack>> {
    let mut vec = Vec::with_capacity(size);
    for _ in 0..size {
        vec.push(Some(ItemStack {
            item: 160,
            stack_size: 1,
            metadata: 15,
            tag_compound: Some(NBT::with_nodes(vec![
                NBT::compound("display", vec![
                    NBT::string("Name", "")
                ])
            ])),
        }))
    }
    vec
}
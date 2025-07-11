use crate::dungeon::dungeon_state::DungeonState;
use crate::net::packets::client_bound::chat::Chat;
use crate::net::packets::packet::SendPacket;
use crate::net::packets::server_bound::click_window::ClickWindow;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::Player;
use crate::server::server::Server;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::nbt::NBT;


// there isn't going to be that many uis 
// so there is no point creating large amounts of abstraction
// but it does hurt my soul

#[derive(Debug, Copy, Clone)]
pub enum UI {
    None,
    // this is here to direct clicks to the actual inventory where all the items are stored, etc.
    Inventory,
    SkyblockMenu,
    MortReadyUpMenu
}

#[derive(Debug)]
pub struct ContainerData {
    pub title: String,
    pub slot_amount: u8,
}

impl UI {

    /// this function returns data for opening a container,
    /// should not be used for UI's that don't use a container
    pub fn get_container_data(&self) -> Option<ContainerData> {
        match self {
            UI::SkyblockMenu => Some(ContainerData {
                title: "SkyBlock Menu".to_string(),
                slot_amount: 54,
            }),
            UI::MortReadyUpMenu => Some(ContainerData {
                title: "Ready Up".to_string(),
                slot_amount: 54,
            }),
            _ => None
        }
    }

    /// handles the click window packet for all UI
    pub fn handle_click_window(
        &self,
        packet: &ClickWindow,
        player: &mut Player,
    ) -> anyhow::Result<()> {
        // todo in wd branch, track active windows with transaction packet sync and stuff
        // to make sure client doesnt send packets for a different gui when it hasn't recieved new 1
        let network_tx = &player.server_mut().network_tx;
        match self {
            // maybe flag, since should never be possible
            UI::None => player.sync_inventory()?,
            UI::Inventory => {
                if packet.slot_id == 44 { 
                    player.open_ui(UI::SkyblockMenu)?;
                    return Ok(())
                }
                if player.inventory.click_slot(&packet, &player.client_id, network_tx)? { 
                    // needs re-syncing
                    player.sync_inventory()?;
                }
            },
            
            UI::SkyblockMenu => {
                player.sync_inventory()?;
                if packet.slot_id == 0 { 
                    Chat {
                        component: ChatComponentTextBuilder::new("clicked hello").build(),
                        typ: 0,
                    }.send_packet(player.client_id, network_tx)?
                }
            }
            UI::MortReadyUpMenu => {
                if packet.slot_id == 13 {
                    let dung = &mut player.server_mut().dungeon;
                    match dung.state {
                        DungeonState::NotReady => {
                            dung.state = DungeonState::Starting { tick_countdown: 120 }
                        }
                        DungeonState::Starting { .. } => {
                            dung.state = DungeonState::NotReady
                        }
                        _ => {}
                    }
                }
                player.sync_inventory()?;
            }
        }
        Ok(())
    }

    /// returns a list of items to send to client 
    pub fn get_container_contents(&self, server: &Server) -> Option<Vec<Option<ItemStack>>> {
        match self {
            UI::SkyblockMenu => {
                let content = default_container_content(54);
                Some(content)
            }
            UI::MortReadyUpMenu => {
                let mut content = default_container_content(54);
                
                let (item_name, color) = if let DungeonState::NotReady = server.dungeon.state {
                    ("§cNot Ready", 14)
                } else {
                    ("§aReady", 13)
                };
                content.insert(13, Some(ItemStack {
                    item: 95,
                    stack_size: 1,
                    metadata: color,
                    tag_compound: Some(NBT::with_nodes(vec![
                        NBT::compound("display", vec![
                            NBT::string("Name", item_name)
                        ])
                    ])),
                }));
                Some(content)
            }
            _ => None
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
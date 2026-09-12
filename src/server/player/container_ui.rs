use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::dungeon_state::DungeonState::NotReady;
use crate::net::protocol::play::clientbound::CloseWindow;
use crate::net::protocol::play::clientbound::SoundEffect;
use crate::net::protocol::play::serverbound::ClickWindow;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::{ClientId, Player};
use crate::server::player::terminal::TerminalType;
use crate::server::player::terminals::select::ENUM_DYE;
use crate::server::player::terminals::starts_with::LETTERS;
use crate::server::server::Server;
use crate::server::utils::nbt::nbt::NBT;
use crate::server::utils::sounds::Sounds;

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
    MortReadyUpMenu,
    CncMenu,
    TerminalUI {
        typ: TerminalType,
        rand: i16
    }
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
            UI::CncMenu => Some(ContainerData {
                title: "Undersized party!".to_string(),
                slot_amount: 36,
            }),
            UI::TerminalUI { typ: TerminalType::Panes, rand } => Some(ContainerData {
                title: "Correct all the panes!".to_string(),
                slot_amount: 45,
            }),
            UI::TerminalUI { typ: TerminalType::Order, rand } => Some(ContainerData {
                title: "Click in order!".to_string(),
                slot_amount: 36,
            }),
            UI::TerminalUI { typ: TerminalType::Rubix, rand } => Some(ContainerData {
                title: "Change all to same color!".to_string(),
                slot_amount: 45,
            }),
            UI::TerminalUI { typ: TerminalType::Select, rand } => Some(ContainerData {
                title: ("Select all the ".to_owned() + &*ENUM_DYE[rand].name + " items!").to_string(),
                slot_amount: 54,
            }),
            UI::TerminalUI { typ: TerminalType::StartsWith, rand } => Some(ContainerData {
                title: ("What starts with: '".to_owned() + LETTERS[*rand as usize % LETTERS.len()] + "'?").to_string(),
                slot_amount: 45,
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
                            NBT::string("Name", &format!("§a{}", player.profile.username)),
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
            UI::CncMenu => {
                // 9 wide x 4 tall (36 slots): row 1 (slots 9-17) and row 3/bottom (slots 27-35),
                // both 0-indexed from the top. Column 4 is the middle column of 9.
                let mut content = default_container_content(36);
                content[13] = Some(undersized_party_head());
                content[31] = Some(ItemStack {
                    item: 166, // Barrier
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
            UI::TerminalUI { typ, rand } => { // matches any
                Option::from(player.current_terminal.as_ref()?.get_contents())
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
            UI::CncMenu => {
                if packet.slot_id == 31 {
                    player.current_ui = UI::None;
                    player.write_packet(&CloseWindow {
                        window_id: player.window_id,
                    });
                    return;
                }
                // Undersized-party head: single-use per menu opening (see
                // `cnc_undersized_used`, reset in `Cnc::run`) - either click button works, no
                // need to check `packet.used_button`.
                //
                // This is SkyBlock's "enter a dungeon right now" action: leave whatever
                // dungeon is currently active (if any) and start a brand new, fully
                // independent one (see `dungeon_switch::switch_dungeon`), then announce the
                // entry once the player is actually standing in it.
                if packet.slot_id == 13 && !player.cnc_undersized_used {
                    player.cnc_undersized_used = true;

                    use crate::server::utils::sounds::MEOW_SOUNDS;
                    use rand::seq::IndexedRandom;
                    if let Some(meow) = MEOW_SOUNDS.choose(&mut rand::rng()) {
                        player.write_packet(&SoundEffect {
                            sound: meow.id(),
                            pos_x: player.position.x,
                            pos_y: player.position.y,
                            pos_z: player.position.z,
                            volume: 1.0,
                            pitch: 1.0,
                        });
                    }

                    if let Err(e) = crate::server::dungeon_switch::switch_dungeon(player.server_mut()) {
                        eprintln!("switch_dungeon failed: {e}");
                    } else {
                        send_dungeon_entry_message(player);
                    }
                }
                // Everything else is purely a display for now - just re-sync so a client-side
                // pickup attempt on the panes/sword snaps back.
                player.sync_inventory();
            }
            UI::TerminalUI { typ, rand } => {
                if let Some(mut terminal) = player.current_terminal.take() { // this take thing is kinda weird, but it works ig
                    if terminal.click_slot(packet, player) {
                        player.current_ui = UI::None;
                        player.current_terminal = None;
                        player.write_packet(&CloseWindow {
                            window_id: player.window_id,
                        });

                        // TERMINAL COMPLETED
                        return;
                    }
                    player.current_terminal = Some(terminal);
                    player.open_ui(UI::TerminalUI { typ: typ.clone(), rand: *rand });
                }
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

/// Mojang profile "textures" property value for the "Undersized party!" head - decodes to
/// `{"textures":{"SKIN":{"url":"http://textures.minecraft.net/texture/1acea2911c2ef31477475e43b27b6fe2906ac1c8e0d84880afb9343ae6532095"}}}`.
const UNDERSIZED_PARTY_SKIN_VALUE: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvMWFjZWEyOTExYzJlZjMxNDc3NDc1ZTQzYjI3YjZmZTI5MDZhYzFjOGUwZDg0ODgwYWZiOTM0M2FlNjUzMjA5NSJ9fX0=";

/// The "Undersized party!" head shown in the CNC menu - matches Hypixel's own display
/// name/lore formatting exactly (each lore line keeps its own color/formatting rather than one
/// color applied to the whole entry).
fn undersized_party_head() -> ItemStack {
    let mut stack = ItemStack {
        item: 397, // skull
        stack_size: 1,
        metadata: 3, // player head - resolves its texture from the embedded SkullOwner
        tag_compound: None,
    };
    stack.set_skull_owner(UNDERSIZED_PARTY_SKIN_VALUE);
    stack.set_display_name("\u{a7}eUndersized party!");
    stack.set_lore(&[
        "\u{a7}7You should party up with \u{a7}f5 players",
        "\u{a7}7for this instance!",
        "",
        "\u{a7}c\u{a7}lTHIS INSTANCE IS BEST",
        "\u{a7}c\u{a7}lWITH A 5 PLAYER PARTY!",
        "",
        "\u{a7}7Your party: \u{a7}bSolo",
        "",
        "\u{a7}eClick to play anyway!",
    ]);
    stack
}

/// The Hypixel-style chat separator line used above/below the dungeon-entry announcement.
/// `§m` (strikethrough) on a run of `-` characters is what renders as the continuous line.
const DUNGEON_ENTRY_SEPARATOR: &str = "\u{a7}b\u{a7}m-----------------------------------------------------";

/// Sends the full 3-line "entered The Catacombs" announcement (separator, message, separator)
/// to every connected player, using `player`'s own username - mirrors the real Hypixel dungeon
/// entry message players see when someone joins/starts an instance.
fn send_dungeon_entry_message(player: &mut Player) {
    let entry_line = format!(
        "\u{a7}2[VIP] \u{a7}a{} \u{a7}eentered \u{a7}aThe Catacombs, Floor VII\u{a7}e!",
        player.profile.username
    );

    for (_, other_player) in &mut player.server_mut().world.players {
        other_player.send_message(DUNGEON_ENTRY_SEPARATOR);
        other_player.send_message(&entry_line);
        other_player.send_message(DUNGEON_ENTRY_SEPARATOR);
    }
}
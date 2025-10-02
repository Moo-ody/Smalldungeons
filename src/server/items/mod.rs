use crate::server::items::etherwarp::{handle_ether_warp, handle_teleport};
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::player::inventory::ItemSlot;
use crate::server::utils::nbt::nbt::{NBTNode, NBT};
use crate::server::utils::nbt::serialize::TAG_COMPOUND_ID;
use indoc::indoc;
use std::collections::HashMap;

pub mod item_stack;
mod ether_transmission;
mod etherwarp;
pub mod ender_pearl;
mod hyperion;
pub mod bonzo_projectile;
pub mod jerry_projectile;



/// List of items available to be used
#[derive(Copy, Debug, Clone, PartialEq)]
pub enum Item {
    SkyblockMenu,
    MagicalMap,
    AspectOfTheVoid,
    DiamondPickaxe,
    SpiritSceptre,
    EnderPearl,
    Hyperion,
    TacticalInsertion,
    SuperboomTNT,
    GoldenAxe,
    Terminator,
    BonzoStaff,
    JerryChineGun,
    VanillaChest,
}

impl Item {

    pub fn on_right_click(&self, player: &mut Player) -> anyhow::Result<()> {
        match self {
            Item::EnderPearl => {
                ender_pearl::on_right_click(player)?;
                // Always restore stack size to prevent consumption
                let hotbar_slot = player.held_slot as usize + 36;
                player.inventory.set_slot(ItemSlot::Filled(Item::EnderPearl, 16), hotbar_slot);
                // Sync inventory to ensure client sees the restored stack
                let _ = player.sync_inventory();
            }
            Item::AspectOfTheVoid => {
                let server = &player.server_mut();
                let world = &server.world;

                if player.is_sneaking {
                    handle_ether_warp(player, world)?; // Etherwarp with DDA algorithm
                } else {
                    handle_teleport(player, &server.network_tx)?; // Ether transmission
                }
            }
            Item::Hyperion => {
                hyperion::on_right_click(player)?;
            }
            Item::SpiritSceptre => {
                // spawn bats, they copy yaw and pitch of player, idk the speed or whatever but
                // when they hit a solid block they blow up in like 10 block radius (or square) or something
            }
            Item::TacticalInsertion => {
                // Mark current location for tactical insertion
                let server = &mut player.server_mut();
                let world = &mut server.world;
                
                // Create tactical insertion marker
                let marker = crate::server::world::TacticalInsertionMarker {
                    client_id: player.client_id,
                    return_tick: world.tick_count + 60, // Return after 3 seconds (20 TPS * 3)
                    origin: player.position,
                    damage_echo_window_ticks: 60,
                    yaw: player.yaw,
                    pitch: player.pitch,
                };
                
                // Schedule sounds to play before return (exact timing from Hypixel test)
                let sounds = vec![
                    // Right-click: fire.ignite
                    crate::server::world::ScheduledSound {
                        due_tick: world.tick_count + 0,
                        sound: crate::server::utils::sounds::Sounds::FireIgnite,
                        volume: 1.0,
                        pitch: 0.75,
                    },
                    // 65ms later: fire.ignite (higher pitch)
                    crate::server::world::ScheduledSound {
                        due_tick: world.tick_count + 1, // 65ms ≈ 1 tick at 20 TPS
                        sound: crate::server::utils::sounds::Sounds::FireIgnite,
                        volume: 1.0,
                        pitch: 1.1,
                    },
                    // 506ms later: note.hat
                    crate::server::world::ScheduledSound {
                        due_tick: world.tick_count + 10, // 506ms ≈ 10 ticks
                        sound: crate::server::utils::sounds::Sounds::NoteHat,
                        volume: 0.8,
                        pitch: 1.21,
                    },
                    // 1002ms later: note.hat
                    crate::server::world::ScheduledSound {
                        due_tick: world.tick_count + 20, // 1002ms ≈ 20 ticks
                        sound: crate::server::utils::sounds::Sounds::NoteHat,
                        volume: 0.85,
                        pitch: 1.33,
                    },
                    // 1513ms later: note.hat
                    crate::server::world::ScheduledSound {
                        due_tick: world.tick_count + 30, // 1513ms ≈ 30 ticks
                        sound: crate::server::utils::sounds::Sounds::NoteHat,
                        volume: 0.9,
                        pitch: 1.44,
                    },
                    // 2007ms later: note.hat
                    crate::server::world::ScheduledSound {
                        due_tick: world.tick_count + 40, // 2007ms ≈ 40 ticks
                        sound: crate::server::utils::sounds::Sounds::NoteHat,
                        volume: 0.95,
                        pitch: 1.57,
                    },
                    // 2505ms later: note.hat
                    crate::server::world::ScheduledSound {
                        due_tick: world.tick_count + 50, // 2505ms ≈ 50 ticks
                        sound: crate::server::utils::sounds::Sounds::NoteHat,
                        volume: 1.0,
                        pitch: 1.7,
                    },
                    // After teleport (500ms later): zombie.remedy
                    crate::server::world::ScheduledSound {
                        due_tick: world.tick_count + 60, // 3000ms + 500ms = 3500ms ≈ 60 ticks
                        sound: crate::server::utils::sounds::Sounds::ZombieRemedy,
                        volume: 0.7,
                        pitch: 1.89,
                    },
                    // 100ms later: zombie.remedy
                    crate::server::world::ScheduledSound {
                        due_tick: world.tick_count + 62, // 3600ms ≈ 62 ticks
                        sound: crate::server::utils::sounds::Sounds::ZombieRemedy,
                        volume: 0.6,
                        pitch: 1.73,
                    },
                    // 177ms later: zombie.remedy
                    crate::server::world::ScheduledSound {
                        due_tick: world.tick_count + 66, // 3777ms ≈ 66 ticks
                        sound: crate::server::utils::sounds::Sounds::ZombieRemedy,
                        volume: 0.5,
                        pitch: 1.57,
                    },
                ];
                
                // Add to world's tactical insertions
                world.tactical_insertions.push((marker, sounds));
                
                // Always restore stack size to prevent consumption
                let hotbar_slot = player.held_slot as usize + 36;
                player.inventory.set_slot(ItemSlot::Filled(Item::TacticalInsertion, 16), hotbar_slot);
                // Sync inventory to ensure client sees the restored stack
                let _ = player.sync_inventory();
            }
            Item::SuperboomTNT => {
                // If air right-click, explode near player position as fallback
                let block_pos = crate::server::block::block_position::BlockPos::new(
                    player.position.x.floor() as i32,
                    player.position.y.floor() as i32,
                    player.position.z.floor() as i32,
                );
                let yaw = player.yaw;
                let dir = ((yaw.rem_euclid(360.0) + 45.0) / 90.0).floor() as i32 % 4; // 0=S,1=W,2=N,3=E (approx)
                let radius = match dir {
                    0 | 3 => 3, // South or East => 3
                    _ => 2,     // North or West => 2
                };
                let _ = player.server_mut().dungeon.superboom_at(block_pos, radius);

                // Always restore stack size to prevent consumption
                let hotbar_slot = player.held_slot as usize + 36;
                player.inventory.set_slot(ItemSlot::Filled(Item::SuperboomTNT, 64), hotbar_slot);
                // Sync inventory to ensure client sees the restored stack
                let _ = player.sync_inventory();
            }
            Item::BonzoStaff => {
                // Bonzo Staff is now handled in packet processing (PlayerBlockPlacement)
                // The ghast sound and projectile spawning happens in shoot_bonzo_projectile()
                // No need to do anything here since it's handled by the packet system
            }
            _ => {}
        }
        Ok(())
    }
    
    /// creates a vanilla item stack, including all nbt data.
    /// 
    /// this is only used for packets, we do not need to store this on the server.
    pub fn get_item_stack(&self) -> ItemStack {
        let mut stack = match self {
            Item::SkyblockMenu => ItemStack {
                item: 399,
                stack_size: 1,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§aSkyBlock Menu §7(Click)"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §7View all of your SkyBlock progress,
                            §7including your Skills, Collections,
                            §7Recipes, and more!

                            §eClick to Open!
                        "#})
                    ]),
                ])),
            },
            Item::MagicalMap => ItemStack {
                item: 358,
                stack_size: 1,
                metadata: 1,
                tag_compound: None,
            },
            Item::AspectOfTheVoid => ItemStack {
                item: 277,
                stack_size: 1,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§6Aspect of the Void"),
                        NBT::list_from_string("Lore", indoc! {r#"

                            §6Ability: Ether Transmission §e§lSNEAK RIGHT CLICK
                            §7Teleport to your targeted block up
                            §7to §a61 blocks §7away
                            §8Soulflow Cost: §30
                            §8Mana Cost: §30

                            §6§l§kU§r§6§l LEGENDARY SWORD §kU
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "ASPECT_OF_THE_VOID"),
                    ]),
                ])),
            },
            Item::DiamondPickaxe => ItemStack {
                item: 278,
                stack_size: 1,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::list("ench", TAG_COMPOUND_ID, vec![
                        NBTNode::Compound({
                            let mut map = HashMap::new();
                            map.insert("lvl".into(), NBTNode::Short(10));
                            map.insert("id".into(), NBTNode::Short(32));
                            map
                        })
                    ]),
                    NBT::compound("display", vec![
                        NBT::list_from_string("Lore", indoc! {r#"
                            §8Breaking Power 4

                            §9Efficiency X
                            §7Increases how quickly your tool
                            §7breaks blocks.

                            §9§l§kE§r§9§l RARE PICKAXE §kE
                        "#}),
                        NBT::string("Name", "§9Diamond Pickaxe"),
                    ]),
                ])),
            },
            Item::SpiritSceptre => ItemStack {
                item: 38,
                stack_size: 1,
                metadata: 2,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§d⚚ Heroic Spirit Sceptre §6✪✪✪✪✪"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §7Gear Score: §d781 §8(3948)
                            §7Damage: §c+242 §e(+30) §8(+1,407.13)
                            §7Strength: §c+80 §e(+30) §9(+50) §8(+504.8)
                            §7Crit Damage: §c+70% §8(+441.7%)
                            §7Bonus Attack Speed: §c+7% §9(+7%) §8(+10.92%)
                            §7Intelligence: §a+518 §9(+125) §d(+30) §8(+3,060.35)
                            §6[§b✎§6]

                            §d§l§d§lSwarm V§9, §9Champion X§9, §9Critical VI
                            §9Ender Slayer VI§9, §9Fire Aspect III§9, §9First Strike IV
                            §9Giant Killer VI§9, §9Lethality VI§9, §9Mana Steal III
                            §9Prosecute V§9, §9Smite VII§9, §9Tabasco III
                            §9Thunderlord VI§9, §9Vampirism VI

                            §b◆ Music Rune III

                            §b§l⦾ §6Ability: Guided Bat  §e§lRIGHT CLICK
                            §7Shoots a guided spirit bat, following your aim
                            §7and exploding for §c6,253.4 §7damage.
                            §8Mana Cost: §3180

                            §8§l* §8Co-op Soulbound §8§l*
                            §d§l§ka§r §d§lMYTHIC DUNGEON SWORD §d§l§ka
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "STARRED_BAT_WAND"),
                        NBT::compound("runes", vec![
                            NBT::string("MUSIC", "3"),
                        ]),
                        NBT::string("modifier", "heroic"),
                        NBT::string("dungeon_item_level", "5"),
                        NBT::string("power_ability_scroll", "SAPPHIRE_POWER_SCROLL"),
                        NBT::string("originTag", "CRAFTING_GRID_COLLECT"),
                        NBT::compound("enchantments", vec![
                            NBT::string("ultimate_swarm", "5"),
                            NBT::string("critical", "6"),
                            NBT::string("smite", "7"),
                            NBT::string("ender_slayer", "6"),
                            NBT::string("telekinesis", "1"),
                            NBT::string("vampirism", "6"),
                            NBT::string("fire_aspect", "3"),
                            NBT::string("giant_killer", "6"),
                            NBT::string("mana_steal", "3"),
                            NBT::string("first_strike", "4"),
                            NBT::string("tabasco", "3"),
                            NBT::string("thunderlord", "6"),
                            NBT::string("champion", "10"),
                            NBT::string("lethality", "6"),
                            NBT::string("PROSECUTE", "5"),
                        ]),
                        NBT::string("uuid", "e1408a2c-4028-4460-a5bf-7391cf5fd0d5"),
                        NBT::string("anvil_uses", "2"),
                        NBT::string("hot_potato_count", "15"),
                        NBT::compound("gems", vec![
                            NBT::compound("SAPPHIRE_0", vec![
                                NBT::string("uuid", "f40fb3a2-0924-404b-8a38-aaea774eb0e4"),
                                NBT::string("quality", "PERFECT"),
                            ]),
                        ]),
                        NBT::string("champion_combat_xp", "2.7896516998197712E7"),
                        NBT::string("donated_museum", "1"),
                        NBT::string("timestamp", "1600850460000"),
                    ]),
                ])),
            },
            Item::EnderPearl => ItemStack {
                item: 368,
                stack_size: 16,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§fEnder Pearl"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §7§8Collection Item

                            §f§lCOMMON
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "ENDER_PEARL"),
                    ]),
                ])),
            },
            Item::Hyperion => ItemStack {
                item: 267,
                stack_size: 1,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§dHeroic Hyperion §6✪✪✪✪✪"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §7Gear Score: §d1218 §8(5000)
                            §7Damage: §c+369 §e(+30) §8(+2,164.33)
                            §7Strength: §c+245 §e(+30) §9(+50) §8(+1,451.3)
                            §7Crit Damage: §c+70% §8(+441.7%)
                            §7Bonus Attack Speed: §c+7% §9(+7%) §8(+10.92%)
                            §7Intelligence: §a+670 §9(+125) §d(+60) §8(+4,006.85)
                            §7Ferocity: §a+33 §8(+46.8)
                            §6[§b✎§6] §6[§b⚔§6]

                            §d§l§d§lUltimate Wise V§9, §9Champion X§9, §9Cleave V
                            §9Critical VI§9, §9Cubism V§9, §9Drain IV
                            §9Ender Slayer VI§9, §9Experience IV§9, §9Fire Aspect III
                            §9First Strike IV§9, §9Giant Killer VI§9, §9Impaling III
                            §9Lethality VI§9, §9Looting IV§9, §9Luck VI
                            §9Prosecute VI§9, §9Scavenger V§9, §9Sharpness VI
                            §9Smite VII§9, §9Tabasco III§9, §9Thunderbolt VII
                            §9Vampirism VI§9, §9Venomous V

                            §7Deals §c+50% §7damage to §8☠ Wither §7mobs.
                            §7Grants §c+1 §c❁ Damage §7and §a+2 §b✎
                            §bIntelligence §7per §cCatacombs §7level.

                            §aScroll Abilities:
                            §c§l⦾ §6Ability: Wither Impact  §e§lRIGHT CLICK
                            §7Teleport §a10 blocks§7 ahead of you.
                            §7Then implode dealing §c20,208.4 §7damage
                            §7to nearby enemies. Also applies the
                            §7wither shield scroll ability reducing
                            §7damage taken and granting an
                            §7absorption shield for §e5 §7seconds.
                            §8Mana Cost: §3135

                            §8§l* §8Co-op Soulbound §8§l*
                            §d§l§ka§r §d§lMYTHIC DUNGEON SWORD §d§l§ka
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "HYPERION"),
                    ]),
                ])),
            },
            Item::TacticalInsertion => ItemStack {
                item: 369,
                stack_size: 16,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§6Tactical Insertion"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §6Ability: Gorilla Tactics  §e§lRIGHT CLICK
                            §7Marks your location and teleport back there
                            §7after §a3s§7.
                            §7
                            §7On coming back, §6burn §7enemies within §b3 §7blocks
                            §7and set your §c❤ Health §7to HALF of what it was.
                            §7
                            §7The §6burn §7deals §c10% §7of ALL damage you dealt
                            §7within the §a3s§7, spread over §66s§7.
                            §8Mana Cost: §3180
                            §8Cooldown: §a20s

                            §6§l§ka§r §6§lLEGENDARY §6§l§ka
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "TACTICAL_INSERTION"),
                    ]),
                ])),
            },
            Item::SuperboomTNT => ItemStack {
                item: 46,
                stack_size: 64,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§9Superboom TNT"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §7Breaks weak walls. Can be used to
                            §7blow up Crypts in §cThe Catacombs §7and
                            §7§5Crystal Hollows§7.

                            §9§lRARE
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "SUPERBOOM_TNT"),
                    ]),
                ])),
            },
            Item::GoldenAxe => ItemStack {
                item: 286,
                stack_size: 1,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::list("ench", TAG_COMPOUND_ID, vec![
                        NBTNode::Compound({
                            let mut map = HashMap::new();
                            map.insert("lvl".into(), NBTNode::Short(5));
                            map.insert("id".into(), NBTNode::Short(32));
                            map
                        })
                    ]),
                    NBT::compound("display", vec![
                        NBT::string("Name", "§fGolden Axe"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §7Damage: §c+20

                            §9Efficiency V
                            §7Increases how quickly your tool
                            §7breaks blocks.

                            §7§8This item can be reforged!
                            §f§lCOMMON AXE
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "GOLD_AXE"),
                    ]),
                ])),
            },
            Item::Terminator => ItemStack {
                item: 261,
                stack_size: 1,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§dPrecise Terminator §6✪✪✪✪✪§c➎"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §7Gear Score: §d1010 §8(4842)
                            §7Damage: §c+371 §e(+30) §8(+2,145.4)
                            §7Strength: §c+124 §e(+30) §6[+5] §9(+34) §8(+750.89)
                            §7Crit Chance: §c+30% §9(+15%) §8(+46.8%)
                            §7Crit Damage: §c+350% §9(+70%) §8(+2,050.75%)
                            §7Bonus Attack Speed: §c+44% §8(+62.4%)
                            §7Shot Cooldown: §a0.5s

                            §d§l§d§lSoul Eater V§9, §9Chance IV§9, §9Dragon Tracer V
                            §9Flame II§9, §9Gravity V§9, §9Impaling III
                            §9Infinite Quiver X§9, §9Overload V§9, §9Piercing I
                            §9Power VI§9, §9Snipe III§9, §9Toxophilite X

                            §7Shoots §b3 §7arrows at once.
                            §7Can damage endermen.

                            §cDivides your §9☣ Crit Chance §cby 4!

                            §6Ability: Salvation  §e§lLEFT CLICK
                            §7Can be cast after landing §63 §7hits.
                            §7§7Shoot a beam, penetrating up to §e5
                            §e§7enemies.
                            §7The beam always crits.
                            §8Soulflow Cost: §3§31⸎

                            §dShortbow: Instantly shoots!

                            §9Precise Bonus
                            §7Deal §a+10% §7extra damage when
                            §7arrows hit the head of a mob.

                            §8§l* §8Co-op Soulbound §8§l*
                            §d§l§ka§r §d§lMYTHIC DUNGEON BOW §d§l§ka
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "TERMINATOR"),
                    ]),
                ])),
            },
            Item::BonzoStaff => ItemStack {
                item: 369, // blaze_rod
                stack_size: 1,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::list("ench", TAG_COMPOUND_ID, vec![]),
                    NBT::compound("display", vec![
                        NBT::string("Name", "§9⚚ Heroic Bonzo's Staff §6✪✪✪✪✪"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §7Gear Score: §d405 §8(2327)
                            §7Damage: §c+176 §8(+1,009.6)
                            §7Strength: §c+25 §9(+25) §8(+157.75)
                            §7Bonus Attack Speed: §e+2% §9(+2%) §8(+3.12%)
                            §7Intelligence: §b+395 §9(+65) §8(+2,303.15)
                             §8[§7✎§8] §8[§8✎§8]

                            §d§l§d§lUltimate Wise V§9, §9Luck II

                            §6Ability: Showtime  §e§lRIGHT CLICK
                            §7Shoots balloons that create a large explosion
                            §7on impact, dealing up to §c22,999.9 §7damage.
                            §8Mana Cost: §341

                            §9§lRARE DUNGEON SWORD
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("modifier", "heroic"),
                        NBT::string("upgrade_level", "5"),
                        NBT::string("id", "STARRED_BONZO_STAFF"),
                        NBT::compound("enchantments", vec![
                            NBT::string("luck", "2"),
                            NBT::string("ultimate_wise", "5"),
                        ]),
                        NBT::string("uuid", "3fe1615b-7d69-47f2-9541-81be655310e5"),
                        NBT::string("timestamp", "1734732834553"),
                    ]),
                ])),
            },
            Item::JerryChineGun => ItemStack {
                item: 418, // golden_horse_armor
                stack_size: 1,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§5Jerry-Chine Gun"),
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "JERRY_STAFF"),
                        NBT::string("type", "SWORD"),
                    ]),
                ])),
            },
            Item::VanillaChest => ItemStack {
                item: 54, // chest
                stack_size: 64,
                metadata: 0,
                tag_compound: None,
            },
        };
        if let Some(ref mut tag) = stack.tag_compound {
            tag.nodes.insert("Unbreakable".into(), NBTNode::Byte(1));
            tag.nodes.insert("HideFlags".into(), NBTNode::Int(127));
        }
        stack
    }
}

use crate::server::items::ether_transmission::handle_teleport;
use crate::server::items::etherwarp::handle_ether_warp;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::player::ui::UI;
use crate::server::utils::nbt::encode::TAG_COMPOUND_ID;
use crate::server::utils::nbt::{NBTNode, NBT};
use crate::net::packets::packet::SendPacket;
use indoc::indoc;

mod etherwarp;
pub mod item_stack;
pub mod ender_pearl;
mod ether_transmission;
mod terminator;
mod hyperion;

/// List of items available to be used
#[derive(Copy, Debug, Clone, PartialEq)]
pub enum Item {
    SkyblockMenu,
    AspectOfTheVoid,
    DiamondPickaxe,
    SpiritSceptre,
    EnderPearl,
    Hyperion,
    TacticalInsertion,
    SuperboomTNT,
    GoldenAxe,
    Terminator,
    DepthStriderBoots,
}


impl Item {

    pub fn on_right_click(&self, player: &mut Player) -> anyhow::Result<()> {
        match self {
            Item::SkyblockMenu => {
                player.open_ui(UI::SkyblockMenu)?
            }
            Item::EnderPearl => {
                ender_pearl::on_right_click(player)?;
            }
            Item::AspectOfTheVoid => {
                let server = &player.server_mut();
                let world = &server.world;

                if player.is_sneaking {
                    handle_ether_warp(player, world)?;
                } else {
                    // Play endermen portal sound when not crouching
                    let _ = crate::net::packets::client_bound::sound_effect::SoundEffect {
                        sounds: crate::server::utils::sounds::Sounds::EndermenPortal,
                        volume: 1.0,
                        pitch: 1.0,
                        x: player.position.x,
                        y: player.position.y,
                        z: player.position.z,
                    }.send_packet(player.client_id, &player.network_tx);
                    
                    handle_teleport(player, &server.network_tx, 12.0)?;
                }
            }
            Item::SpiritSceptre => {
                // spawn bats, they copy yaw and pitch of player, idk the speed or whatever but
                // when they hit a solid block they blow up in like 10 block radius (or square) or something
            }
            Item::Hyperion => {
                // Wither Impact: teleport up to 10 blocks and implode with 10-block radius
                hyperion::on_right_click(player)?;
            }
            Item::TacticalInsertion => {
                // Mark current location; world tick loop will return after 3s
                let world = &mut player.server_mut().world;
                let return_after_ticks = 3 * 20; // 3 seconds
                let marker = crate::server::world::TacticalInsertionMarker {
                    client_id: player.client_id,
                    return_tick: world.tick_count + return_after_ticks,
                    origin: player.position,
                    damage_echo_window_ticks: 3 * 20,
                    yaw: player.yaw,
                    pitch: player.pitch,
                };
                // sound timeline relative to now
                let base = world.tick_count;
                let schedule = [
                    (0u64,  crate::server::utils::sounds::Sounds::FireIgnite, 1.00f32, 0.746032f32),
                    (52,    crate::server::utils::sounds::Sounds::FireIgnite, 1.00f32, 1.095238f32),
                    (502,   crate::server::utils::sounds::Sounds::NoteHat,    0.80f32, 1.206349f32),
                    (999,   crate::server::utils::sounds::Sounds::NoteHat,    0.85f32, 1.333333f32),
                    (1503,  crate::server::utils::sounds::Sounds::NoteHat,    0.90f32, 1.444444f32),
                    (2007,  crate::server::utils::sounds::Sounds::NoteHat,    0.95f32, 1.571429f32),
                    (2507,  crate::server::utils::sounds::Sounds::NoteHat,    1.00f32, 1.698413f32),
                    (2958,  crate::server::utils::sounds::Sounds::ZombieRemedy, 0.70f32, 1.888889f32),
                    (3002,  crate::server::utils::sounds::Sounds::NoteHat,       1.05f32, 1.809524f32),
                    (3120,  crate::server::utils::sounds::Sounds::ZombieRemedy, 0.60f32, 1.730159f32),
                    (3251,  crate::server::utils::sounds::Sounds::ZombieRemedy, 0.50f32, 1.571429f32),
                ];
                let sounds: Vec<crate::server::world::ScheduledSound> = schedule.iter().map(|(ms, s, v, p)| {
                    let ticks = (ms / 50) as u64; // 20 tps approx
                    crate::server::world::ScheduledSound { due_tick: base + ticks, sound: *s, volume: *v, pitch: *p }
                }).collect();
                world.tactical_insertions.push((marker, sounds));
            }
            Item::Terminator => {
                terminator::on_right_click(player)?;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn on_left_click(&self, _player: &mut Player) -> anyhow::Result<()> {
        // Placeholder: no-op unless we add melee interactions per item
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
                Item::SpiritSceptre => ItemStack {
                item: 38, // 38 is the Minecraft ID for red flower
                stack_size: 1,
                metadata: 2, // 2 is the data value for poppy, adjust as needed
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§5Spirit Sceptre"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §7Shoots a spirit bat that explodes
                            §7on impact, dealing §carea damage§7.

                            §6Ability: Spirit Bomb §e§lRIGHT CLICK
                            §7Launches a spirit bat that explodes
                            §7on contact with enemies or blocks.

                            §8Mana Cost: §b250

                            §5§lEPIC WEAPON
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "SPIRIT_SCEPTRE"),
                    ]),
                ])),
            },
                        Item::EnderPearl => ItemStack {
                item: 368,
                stack_size: 16,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "Ender Pearl"),
                        NBT::list_from_string("Lore", indoc! {r#"

                            Ender Pearl

                            its true
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "Ender_Pearl"),
                    ]),
                ])),
            },
            Item::DiamondPickaxe => ItemStack {
                item: 278,
                stack_size: 1,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§9Diamond Pickaxe"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §8Breaking Power 4

                            §9Efficiency X
                            §7Increases how quickly your tool
                            §7breaks blocks.

                            §9§l§kE§r§9§l RARE PICKAXE §kE
                        "#})
                    ]),
                    NBT::list("ench", TAG_COMPOUND_ID, vec![
                        NBTNode::Compound(vec![
                            NBT::short("id", 32),
                            NBT::short("lvl", 10),
                        ])
                    ])
                ])),
            },
            Item::Hyperion => ItemStack {
    item: 267,         
    stack_size: 1,
    metadata: 0,
    tag_compound: Some(NBT::with_nodes(vec![
        NBT::compound("display", vec![
            NBT::string("Name", "§dHeroic Hyperion §6✪§6✪§6✪§6✪§6✪"),
            NBT::list_from_string("Lore", indoc! {r#"
§7Gear Score: §a12065 §8(§71050§8)
§7Damage: §c+359 §8(+30)
§7Strength: §c+245 §8(+30)
§7Crit Damage: §9+70% §8(+40%)
§7Intelligence: §b+670 §8(+125)
§7Ferocity: §c+33 §8(+16.8)

§9Ultimate Wise §b5§7, §9Champion §b10§7, §9Cleave §b5
§9Critical §b6§7, §9Cubism §b5§7, §9Ender Slayer §b6
§9Experience §b4§7, §9First Strike §b4§7, §9Giant Killer §b6
§9Impaling §b3§7, §9Lethality §b6§7, §9Looting §b4
§9Luck §b6§7, §9Prosecute §b6§7, §9Scavenger §b5
§9Smite §b7§7, §9Thunderlord §b7§7, §9Vampirism §b6§7, §9Venomous §b5§7, §9Drain §b1

§7Deals §c+50% §7damage to §8Wither §7mobs.
§7Grants §a+1 §7Damage and §b+2 §7Intelligence per Catacombs level.

§6Scroll Ability: §e§lRIGHT CLICK §6Wither Impact
§7Teleport §a10 §7blocks ahead, then implode dealing §c10,000 §7damage
§7to nearby mobs and grant absorption, damage reduction and intel for §a5s.

§8Catacombs Requirement: §a28
§8Mana Cost: §b135

§d§lMYTHIC DUNGEON SWORD §8§kL
"#}),
        ]),
        NBT::compound("ExtraAttributes", vec![
            NBT::string("id", "HYPERION"),
        ]),
        // Optional vanilla enchants (1.8 format):
        // NBT::list("ench", vec![
        //     NBT::compound("", vec![ NBT::short("id", 16), NBT::short("lvl", 5) ]), // Sharpness V
        // ]),
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
                §6Ability: Gorilla Tactics §e§lRIGHT CLICK
                §7Marks your location and teleport back there
                §7after §a3s§7.

                §7On coming back, §cburn §7enemies within §a3 blocks
                §7and set your §c❤ Health §7to HALF of what it was.

                §7The §cburn §7deals §c10% §7of ALL damage you dealt
                §7within the §a3s§7, spread over §a6s§7.
                §8Mana Cost: §b180
                §8Cooldown: §a20s

                §d§lRARITY UPGRADED
                §6§lLEGENDARY §f
            "#}),
        ]),
        NBT::compound("ExtraAttributes", vec![
            NBT::string("id", "TACTICAL_INSERTION"),
        ]),
    ])),
},
Item::SuperboomTNT => ItemStack {
    item: 46, // 46 = Minecraft TNT
    stack_size: 64,
    metadata: 0,
    tag_compound: Some(NBT::with_nodes(vec![
        NBT::compound("display", vec![
            NBT::string("Name", "§9Superboom TNT"),
            NBT::list_from_string("Lore", indoc! {r#"
                §7Breaks weak walls. Can be used to
                §7blow up Crypts in §cThe Catacombs §7and
                §cCrystal Hollows§7.

                §9RARE
            "#}),
        ]),
        NBT::compound("ExtraAttributes", vec![
            NBT::string("id", "SUPERBOOM_TNT"),
        ]),
    ])),
},
Item::GoldenAxe => ItemStack {
    item: 286, // golden axe id (1.8)
    stack_size: 1,
    metadata: 0,
    tag_compound: Some(NBT::with_nodes(vec![
        NBT::compound("display", vec![
            NBT::string("Name", "§6Golden Axe"),
            NBT::list_from_string("Lore", indoc! {r#"
                §7A shiny axe for testing.
                §7Placed in the top middle slot.
                
                §9Efficiency V
                §7Increases how quickly your tool
                §7breaks blocks.
            "#})
        ]),
        NBT::compound("ExtraAttributes", vec![
            NBT::string("id", "GOLDEN_AXE"),
        ]),
        // Vanilla enchants (1.8 format)
        NBT::list("ench", TAG_COMPOUND_ID, vec![
            NBTNode::Compound(vec![
                NBT::short("id", 32), // Efficiency
                NBT::short("lvl", 5),
            ])
        ])
    ])),
},
Item::Terminator => ItemStack {
    item: 261, // bow id
    stack_size: 1,
    metadata: 0,
    tag_compound: Some(NBT::with_nodes(vec![
        NBT::compound("display", vec![
            NBT::string("Name", "§dPrecise Terminator §6✪§6✪§6✪§6✪§6✪"),
            NBT::list_from_string("Lore", indoc! {r#"
§7Gear Score: §a1010 §8(§7492§8)
§7Damage: §c+371 §8(+30) §7(§c+2,145§7)
§7Strength: §c+124 §8(+34) §7(§c+750.89§7)
§7Crit Chance: §9+30% §8(+15%) §7(§915.8%§7)
§7Crit Damage: §9+53% §8(+7%) §7(§9+2,503.75%§7)
§7Bonus Attack Speed: §a+44% §8(+62.1%)
§7Shot Cooldown: §60.5s

§9Soul Eater §65, §9Chance §64
§9Dragon Tracer §65, §9Flame §6 2, §9Impaling §63
§9Infinite Quiver §6 10, §9Overload §6 5, §9Piercing §61
§9Power §6 6, §9Snipe §6 3, §9Gravity §6 5
§9Toxophilite §6 10

§7Shoots §a3 §7arrows at once.
§7Can damage endermen.

§6Divides your §9✧ Crit Chance §6by §c4§6!

§6Ability: Salvation §e§lRIGHT CLICK
§7Can be cast after landing §a3 §7hits.
§7Shoot a beam, penetrating up to §a5 §7enemies.
§7The beam always crits.

§aPrecise Bonus
§7Deal §a+10% §7extra damage when
§7arrows hit the head of a mob.

§d§lMYTHIC DUNGEON BOW §9
            "#})
        ]),
        NBT::compound("ExtraAttributes", vec![
            NBT::string("id", "TERMINATOR"),
        ]),
    ])),
},


            Item::DepthStriderBoots => ItemStack {
                item: 301, // leather boots id
                stack_size: 1,
                metadata: 0,
                tag_compound: Some(NBT::with_nodes(vec![
                    NBT::compound("display", vec![
                        NBT::string("Name", "§cNecrotic Storm's Boots §6✪§6✪§6✪§6✪"),
                        NBT::list_from_string("Lore", indoc! {r#"
                            §7Color: §c#901A32
                            §7Gear Score: §a858 §8(§74880§8)
                            
                            §7Health: §a+294.5 §8(+60) §7(§a+1,766.8§7)
                            §7Defense: §a+121.5 §8(+30) §7(§a+725.65§7)
                            §7Speed: §a+6 §8(+37.86§7)
                            §7Intelligence: §a+535 §8(+200) (+60) §7(§a+3,218.1§7)
                            §7Health Regen: §a+10 §8(+63.1§7)
                            
                            §7Enchantments:
                            §9Wisdom 5
                            §9Depth Strider 3
                            §9Feather Falling 10
                            §9Growth 5
                            §9Protection 5
                            §9Rejuvenate 5
                            §9Sugar Rush 3
                            §9Thorns 3
                            
                            §7Special Effect: §cReduces the damage you take from withers by 10%.
                            
                            §7Full Set Bonus: §6Witherborn §8(3/4)
                            §7Spawns a wither minion every 30 seconds up to a maximum 1 wither.
                            §7Your withers will travel to and explode on nearby enemies.
                            
                            §cRose Dyed
                            §6RARITY UPGRADED
                            §6> MYTHIC DUNGEON BOOTS
                        "#})
                    ]),
                    NBT::compound("ExtraAttributes", vec![
                        NBT::string("id", "WISE_WITHER_BOOTS"),
                    ]),
                    // Vanilla enchants including Depth Strider 3
                    NBT::list("ench", TAG_COMPOUND_ID, vec![
                        NBTNode::Compound(vec![
                            NBT::short("id", 8), // Depth Strider
                            NBT::short("lvl", 3),
                        ]),
                        NBTNode::Compound(vec![
                            NBT::short("id", 2), // Feather Falling
                            NBT::short("lvl", 10),
                        ]),
                        NBTNode::Compound(vec![
                            NBT::short("id", 0), // Protection
                            NBT::short("lvl", 5),
                        ]),
                        NBTNode::Compound(vec![
                            NBT::short("id", 17), // Thorns
                            NBT::short("lvl", 3),
                        ])
                    ])
                ])),
            },
        };
        if let Some(ref mut tag) = stack.tag_compound {
            tag.nodes.push(NBT::byte("Unbreakable", 1));
            tag.nodes.push(NBT::int("HideFlags", 127));
        }
        stack
    }
}

// use crate::server::commands::argument::Argument;
// use crate::server::commands::command::CommandMetadata;
// use crate::server::commands::outcome::Outcome;
// use crate::server::player::player::Player;
// use crate::server::world::World;
// use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
// use crate::server::utils::color::MCColors;
// use crate::net::protocol::play::clientbound::{Chat, PositionLook};

// pub struct P3;

// fn spawn_p3_armor_stands(world: &mut World) {
//     let terminals = world.p3_manager.get_terminals_to_spawn();
//     let devices = world.p3_manager.get_devices_to_spawn();
    
//     for mut terminal in terminals {
//         terminal.spawn_armor_stands(world);
//     }
    
//     for mut device in devices {
//         device.spawn_armor_stands(world);
//     }
// }

// impl CommandMetadata for P3 {
//     const NAME: &'static str = "p3";

//     fn run(world: &mut World, player: &mut Player, args: &[&str]) -> anyhow::Result<Outcome> {
//         if args.is_empty() {
//             // Send PositionLook packet to teleport the player to P3 area (Simon Says coordinates)
//             player.write_packet(&PositionLook {
//                 x: 110.5,
//                 y: 121.0,
//                 z: 92.5,
//                 yaw: 0.0,
//                 pitch: 0.0,
//                 flags: 24, // xyz absolute, yaw/pitch relative (0.0 means no change to view angles)
//             });
            
//             // Start P3
//             world.p3_manager.start();
//             spawn_p3_armor_stands(world);
            
//             // Send boss message
//             let boss_message = ChatComponentTextBuilder::new(crate::dungeon::p3::p3_manager::P3Manager::get_boss_message())
//                 .color(MCColors::DarkRed)
//                 .bold()
//                 .build();
            
//             player.write_packet(&Chat {
//                 component: boss_message,
//                 chat_type: 0
//             });
            
//             let message = ChatComponentTextBuilder::new("P3 started! Teleported to P3 area.")
//                 .color(MCColors::Green)
//                 .build();
            
//             player.write_packet(&Chat {
//                 component: message,
//                 chat_type: 0
//             });
//         } else {
//             match args[0] {
//                 "s1" => {
//                     world.p3_manager.start_section(crate::dungeon::p3::p3_manager::Section::S1);
//                     spawn_p3_armor_stands(world);
                    
//                     let message = ChatComponentTextBuilder::new("P3 Section 1 started!")
//                         .color(MCColors::Green)
//                         .build();
                    
//                     player.write_packet(&Chat {
//                         component: message,
//                         chat_type: 0
//                     });
//                 }
//                 "s2" => {
//                     world.p3_manager.start_section(crate::dungeon::p3::p3_manager::Section::S2);
//                     spawn_p3_armor_stands(world);
                    
//                     let message = ChatComponentTextBuilder::new("P3 Section 2 started!")
//                         .color(MCColors::Green)
//                         .build();
                    
//                     player.write_packet(&Chat {
//                         component: message,
//                         chat_type: 0
//                     });
//                 }
//                 "s3" => {
//                     world.p3_manager.start_section(crate::dungeon::p3::p3_manager::Section::S3);
//                     spawn_p3_armor_stands(world);
                    
//                     let message = ChatComponentTextBuilder::new("P3 Section 3 started!")
//                         .color(MCColors::Green)
//                         .build();
                    
//                     player.write_packet(&Chat {
//                         component: message,
//                         chat_type: 0
//                     });
//                 }
//                 "s4" => {
//                     world.p3_manager.start_section(crate::dungeon::p3::p3_manager::Section::S4);
//                     spawn_p3_armor_stands(world);
                    
//                     let message = ChatComponentTextBuilder::new("P3 Section 4 started!")
//                         .color(MCColors::Green)
//                         .build();
                    
//                     player.write_packet(&Chat {
//                         component: message,
//                         chat_type: 0
//                     });
//                 }
//                 "reset" => {
//                     world.p3_manager.reset_terminals();
                    
//                     let message = ChatComponentTextBuilder::new("P3 terminals reset!")
//                         .color(MCColors::Yellow)
//                         .build();
                    
//                     player.write_packet(&Chat {
//                         component: message,
//                         chat_type: 0
//                     });
//                 }
//                 "progress" => {
//                     let progress = world.p3_manager.get_term_progress();
//                     let message = ChatComponentTextBuilder::new(format!("P3 Progress: {}/{} terminals completed", progress.0, progress.1))
//                         .color(MCColors::Aqua)
//                         .build();
                    
//                     player.write_packet(&Chat {
//                         component: message,
//                         chat_type: 0
//                     });
//                 }
//                 _ => {
//                     let message = ChatComponentTextBuilder::new("Usage: /p3 [s1|s2|s3|s4|reset|progress]")
//                         .color(MCColors::Red)
//                         .build();
                    
//                     player.write_packet(&Chat {
//                         component: message,
//                         chat_type: 0
//                     });
//                 }
//             }
//         }
        
//         Ok(Outcome::Success)
//     }

//     fn arguments(_: &mut World, _: &mut Player) -> Vec<Argument> {
//         vec![
//             Argument::new("section", false, vec!["s1".to_string(), "s2".to_string(), "s3".to_string(), "s4".to_string(), "reset".to_string(), "progress".to_string()])
//         ]
//     }
// }

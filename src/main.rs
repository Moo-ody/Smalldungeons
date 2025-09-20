mod dungeon;
mod net;
mod server;
mod utils;

use crate::dungeon::door::DoorType;
use crate::dungeon::dungeon::Dungeon;
use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::room::room_data::{RoomData, RoomType};
use crate::dungeon::room::room::Room;
use crate::net::internal_packets::{MainThreadMessage, NetworkThreadMessage};
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound;
use crate::net::protocol::play::clientbound::AddEffect;
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::net::run_network::run_network_thread;
use crate::net::var_int::VarInt;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::chunk::chunk::Chunk;
use crate::server::chunk::chunk_grid::ChunkDiff;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::lava_boost::apply_lava_boost;
use crate::server::player::container_ui::UI;
use crate::server::player::player::Player;
use crate::server::player::scoreboard::ScoreboardLines;
use crate::server::server::Server;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::VIEW_DISTANCE;
use crate::utils::hasher::deterministic_hasher::DeterministicHashMap;
use crate::utils::seeded_rng::SeededRng;
use anyhow::Result;
use chrono::Local;
use include_dir::include_dir;
use indoc::formatdoc;
use rand::seq::IndexedRandom;
use std::collections::HashMap;
use std::env;
use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc::unbounded_channel;

#[tokio::main]
async fn main() -> Result<()> {
    let (network_tx, network_rx) = unbounded_channel::<NetworkThreadMessage>();
    let (main_tx, mut main_rx) = unbounded_channel::<MainThreadMessage>();

    let args: Vec<String> = env::args().collect();

    let rooms_dir = include_dir!("../Evensmallerdungeonsdata/room_data/");

    // roomdata first digit (the key) is just a list of numbers 0..etc. this could just be a vec with roomid lookups.
    let room_data_storage: DeterministicHashMap<usize, RoomData> = rooms_dir
        .entries()
        .iter()
        .filter_map(|file| {
            let file = file.as_file()?;
            let contents = file.contents_utf8()?;
            let name = file.path().file_name()?.to_str()?;
            let room_data = RoomData::from_raw_json(contents);

            let name_parts: Vec<&str> = name.split(",").collect();
            let room_id = name_parts.first()?.parse::<usize>().ok()?;

            Some((room_id, room_data))
        })
        .collect();

    // Load lever data - using include_str for now since the directory name has spaces
    let _lever_json_data = include_str!("room_data/lever shi/lever.json");

    // Might be a good idea to make a new format for storing doors so that indexes etc don't need to be hard coded.
    // But this works for now...
    let door_data: Vec<Vec<Blocks>> = include_str!("door_data/doors.txt")
        .split("\n")
        .map(|line| {
            let mut blocks: Vec<Blocks> = Vec::new();

            for i in (0..line.len() - 1).step_by(4) {
                if let Some(substr) = line.get(i..i + 4) {
                    if let Ok(state) = u16::from_str_radix(substr, 16) {
                        blocks.push(Blocks::from(state));
                    }
                }
            }

            blocks
        })
        .collect();

    let door_type_blocks: HashMap<DoorType, Vec<Vec<Blocks>>> = HashMap::from_iter(
        vec![
            (DoorType::BLOOD, vec![door_data[0].clone()]),
            (DoorType::ENTRANCE, vec![door_data[1].clone()]),
            (DoorType::WITHER, vec![
                door_data[2].clone(),
                door_data[3].clone(),
                door_data[4].clone(),
            ]),
            (
                DoorType::NORMAL,
                vec![
                    door_data[1].clone(),
                    door_data[2].clone(),
                    door_data[3].clone(),
                    door_data[4].clone(),
                    door_data[5].clone(),
                    door_data[6].clone(),
                    door_data[7].clone(),
                ],
            ),
        ]
        .into_iter(),
    );

    let dungeon_strings = include_str!("dungeon_storage/dungeons.txt")
        .split("\n")
        .collect::<Vec<&str>>();

    // Check if a custom dungeon str has been given via cli args

    // let dungeon_str = "080809010400100211121300101415161304171418161300191403161304191905160600919999113099910991099909090099999919990929999999099999999009";

    let dungeon_str = match args.len() {
        0..=1 => {
            let mut rng = rand::rng();
            dungeon_strings.choose(&mut rng).unwrap_or(&"080809010400100211121300101415161304171418161300191403161304191905160600919999113099910991099909090099999919990929999999099999999009")
        }
        _ => args.get(1).map(|s| s.as_str()).unwrap_or("080809010400100211121300101415161304171418161300191403161304191905160600919999113099910991099909090099999919990929999999099999999009"),
    };
    println!("Dungeon String: {}", dungeon_str);

    let rng_seed: u64 = rand::random(); // using a second seed for rng enables the same layout to have randomized rooms. Maybe should be included in the dungeon seed string?
    // let rng_seed: u64 = 12946977352813673410;

    println!("Rng Seed: {}", rng_seed);
    SeededRng::set_seed(rng_seed);

    let mut dungeon = Dungeon::from_str(dungeon_str, &room_data_storage)?;
    
    let mut server = Server::initialize_with_dungeon(network_tx, dungeon);
    server.world.server = &mut server;
    server.dungeon.server = &mut server;

    let mut tick_interval = tokio::time::interval(Duration::from_millis(50));
    tokio::spawn(run_network_thread(
        network_rx,
        server.network_tx.clone(),
        main_tx,
    ));

    // Load the bossroom at fixed coordinates first
    {
        // Load the bossroom JSON data
        let bossroom_json = include_str!("room_data/146,bossroom,-8,-8.json");
        let bossroom_data = RoomData::from_raw_json(bossroom_json);
        
        // Extract dimensions before using bossroom_data
        let bossroom_width = bossroom_data.width;
        let bossroom_length = bossroom_data.length;
        let bossroom_height = bossroom_data.height;
        
        // Store boss room dimensions in dungeon for detection
        server.dungeon.boss_room_width = bossroom_width;
        server.dungeon.boss_room_length = bossroom_length;
        server.dungeon.boss_room_height = bossroom_height;
        
        // Create a single segment for the bossroom at the specified coordinates
        let bossroom_segments = vec![crate::dungeon::room::room::RoomSegment {
            x: 0, // This will be overridden by our custom positioning
            z: 0, // This will be overridden by our custom positioning
            neighbours: [None; 4],
        }];
        
        // Create the bossroom with North rotation (no rotation)
        let bossroom = Room::new(
            bossroom_segments,
            &[], // empty doors array
            bossroom_data,
        );
        
        // Override the corner position to spawn at -18, 255, 4
        // We need to manually load the bossroom since it's not part of the regular dungeon grid
        let corner = BlockPos { x: -18, y: 255, z: 4 };
        
        // Update the dungeon's boss room corner to match the actual loaded position
        server.dungeon.boss_room_corner = corner;
        
        // Manually load the bossroom blocks at the specified position
        for (i, block) in bossroom.room_data.block_data.iter().enumerate() {
            if *block == Blocks::Air {
                continue;
            }
            
            let block = block.clone();
            // No rotation needed since we're placing it directly
            
            let ind = i as i32;
            let x = ind % bossroom.room_data.width;
            let z = (ind / bossroom.room_data.width) % bossroom.room_data.length;
            let y = bossroom.room_data.bottom + ind / (bossroom.room_data.width * bossroom.room_data.length);
            
            // Place the block at the world position
            server.world.set_block_at(block, corner.x + x, y, corner.z + z);
        }
        
        println!("Bossroom loaded at coordinates: x={}, y={}, z={} with dimensions: {}x{}x{}", 
            corner.x, corner.y, corner.z, 
            server.dungeon.boss_room_width, 
            server.dungeon.boss_room_length, 
            server.dungeon.boss_room_height);
        
        // Send bossroom chunks to all connected players
        let bossroom_chunk_x_min = corner.x >> 4;
        let bossroom_chunk_z_min = corner.z >> 4;
        let bossroom_chunk_x_max = (corner.x + bossroom.room_data.width) >> 4;
        let bossroom_chunk_z_max = (corner.z + bossroom.room_data.length) >> 4;
        
        for player in server.world.players.values_mut() {
            for chunk_x in bossroom_chunk_x_min..=bossroom_chunk_x_max {
                for chunk_z in bossroom_chunk_z_min..=bossroom_chunk_z_max {
                    if let Some(chunk) = server.world.chunk_grid.get_chunk(chunk_x, chunk_z) {
                        player.write_packet(&chunk.get_chunk_data(chunk_x, chunk_z, true));
                    }
                }
            }
        }
        
        println!("Bossroom chunks sent to all players");
    }

    let dungeon = &mut server.dungeon;
    
    for room in &mut dungeon.rooms {
        // println!("Room: {:?} type={:?} rotation={:?} shape={:?} corner={:?}", room.segments, room.room_data.room_type, room.rotation, room.room_data.shape, room.get_corner_pos());
        room.load_into_world(&mut server.world);

        // Immediately scan crypts on world load for debug visibility
        if room.crypt_patterns.len() > 0 && !room.crypts_checked {
            let count = room.detect_crypts(&server.world);
            if count == 0 {
                room.debug_crypt_mismatch(&server.world);
            }
        }

        // Set the spawn point to be inside of the spawn room
        if room.room_data.room_type == RoomType::Entrance {
            server.world.set_spawn_point(
                room.get_world_block_pos(&BlockPos {
                    x: 15,
                    y: 72,
                    z: 18,
                })
                .as_dvec3()
                .add_x(0.5)
                .add_z(0.5),
                180.0.rotate(room.rotation),
                0.0,
            );

            // test
            pub struct MortImpl;
            
            impl EntityImpl for MortImpl {
                fn tick(&mut self, _: &mut Entity, _: &mut PacketBuffer) {
                    // rotate
                }
                fn interact(&mut self, _: &mut Entity, player: &mut Player, action: &EntityInteractionType) {
                    if action == &EntityInteractionType::InteractAt {
                        return;
                    }
                    player.open_ui(UI::MortReadyUpMenu);
                }
            }
            
            let id = server.world.spawn_entity(
                room.get_world_block_pos(&BlockPos { x: 15, y: 69, z: 4 })
                    .as_dvec3()
                    .add(DVec3::new(0.5, 0.0, 0.5)),
                EntityMetadata::new(EntityVariant::Zombie { is_child: false, is_villager: false }),
                MortImpl,
            )?;
            if let Some((entity, _)) = server.world.entities.get_mut(&id) {
                entity.yaw = 0.0.rotate(room.rotation);
            }
        }
    }
    
    // Lever system is now integrated into room generation (like crypts and superboom walls)

    for door in &dungeon.doors {
        door.load_into_world(&mut server.world, &door_type_blocks);
    }

    // let zombie_spawn_pos = DVec3 {
    //     x: 25.0,
    //     y: 69.0,
    //     z: 25.0,
    // };

    // let zombie = Entity::create_at(EntityType::Zombie, zombie_spawn_pos, server.world.new_entity_id());
    // let path = Pathfinder::find_path(&zombie, &BlockPos { x: 10, y: 69, z: 10 }, &server.world)?;

    // server.world.entities.insert(zombie.entity_id, zombie);

    let cata_line = ChatComponentTextBuilder::new("")
        .append(
            ChatComponentTextBuilder::new("Dungeon: ")
                .color(MCColors::Aqua)
                .bold()
                .build(),
        )
        .append(
            ChatComponentTextBuilder::new("Catacombs")
                .color(MCColors::Gray)
                .build(),
        )
        .build();

    server.world.player_info.set_line(0, cata_line);

    loop {
        tick_interval.tick().await;
        // let start = std::time::Instant::now();

        while let Ok(message) = main_rx.try_recv() {
            server.process_event(message).unwrap_or_else(|err| eprintln!("Error processing event: {err}"));
        }

        server.dungeon.tick()?;
        server.world.tick()?;

        // for entity_id in server.world.entities.keys().cloned().collect::<Vec<_>>() {
        //     if let Some(mut entity) = server.world.entities.remove(&entity_id) {
        //         entity.ticks_existed += 1;
        //         // this may at some point be abused to prevent getting an entities own self if it iterates over world entities so be careful if you change this
        //         let returned = entity.update(&mut server.world, &server.network_tx);
        //         server.world.entities.insert(entity_id, returned);
        //     }
        // }

        let mut i: usize = 0;
        while i < server.tasks.len() {
            if server.tasks[i].run_in == 0 {
                let task = server.tasks.remove(i);
                (task.callback)(&mut server);
                // index isnt incremented since this entry was removed, shifting the next entry into its place.
            } else {
                server.tasks[i].run_in -= 1;
                i += 1;
            }
        }

        let tab_list_packet = server.world.player_info.get_packet();

        // this needs to be changed to work with loaded chunks, tracking last sent data per player (maybe), etc.
        // also needs to actually be in a vanilla adjacent way.
        for player in server.world.players.values_mut() {
            player.ticks_existed += 1;
            player.write_packet(&clientbound::ConfirmTransaction {
                window_id: 0,
                action_number: -1,
                accepted: false,
            });

            let chunk_x = (player.position.x.floor() as i32) >> 4;
            let chunk_z = (player.position.z.floor() as i32) >> 4;
            let last_chunk_x = (player.last_position.x.floor() as i32) >> 4;
            let last_chunk_z = (player.last_position.z.floor() as i32) >> 4;

            let delta = (chunk_x - last_chunk_x, chunk_z - last_chunk_z);

            if delta.0 != 0 || delta.1 != 0 {
                server.world.chunk_grid.for_each_diff(
                    (chunk_x, chunk_z),
                    (last_chunk_x, last_chunk_z),
                    VIEW_DISTANCE as i32,
                    |x, z, diff| match diff {
                        ChunkDiff::New => {
                            if let Some(chunk) = player.world_mut().chunk_grid.get_chunk_mut(x, z) {
                                player.write_packet(&chunk.get_chunk_data(x, z, true));
                                // Collect valid entity IDs first
                                let valid_entity_ids: Vec<_> = chunk.entities.iter()
                                    .filter(|&&entity_id| server.world.entities.contains_key(&entity_id))
                                    .copied()
                                    .collect();
                                
                                // Process valid entities
                                for &entity_id in &valid_entity_ids {
                                    if let Some((entity, entity_impl)) = server.world.entities.get_mut(&entity_id) {
                                        let buffer = &mut chunk.packet_buffer;
                                        entity.write_spawn_packet(buffer);
                                        entity_impl.spawn(entity, buffer);
                                    }
                                }
                                
                                // Update chunk entities to only contain valid ones
                                chunk.entities.clear();
                                chunk.entities.extend(valid_entity_ids);
                            } else {
                                let chunk_data = Chunk::new().get_chunk_data(x, z, true);
                                player.write_packet(&chunk_data)
                            };
                        }
                        ChunkDiff::Old => {
                            let chunk_data = Chunk::new().get_chunk_data(x, z, true);
                            player.write_packet(&chunk_data)
                        }
                    },
                );
            }

            {
                let view_distance = VIEW_DISTANCE as i32;
                let min_x = chunk_x - view_distance;
                let min_z = chunk_z - view_distance;
                let max_x = chunk_x + view_distance;
                let max_z = chunk_z + view_distance;

                for x in min_x..=max_x {
                    for z in min_z..=max_z {
                        if let Some(chunk) = player.world_mut().chunk_grid.get_chunk(x, z) {
                            player.packet_buffer.copy_from(&chunk.packet_buffer);
                        }
                    }
                }
            }

            let mut sidebar_lines = ScoreboardLines(Vec::new());

            // maybe match time with hypixel,
            let now = Local::now();
            let date = now.format("%m/%d/%y").to_string();
            let time = now.format("%-I:%M%P").to_string();

            let current_skyblock_month = {
                const SKYBLOCK_EPOCH_START_MILLIS: u64 = 1_559_829_300_000;
                const SKYBLOCK_YEAR_MILLIS: u64 = 124 * 60 * 60 * 1000;
                const SKYBLOCK_MONTH_MILLIS: u64 = SKYBLOCK_YEAR_MILLIS / 12;
                const SKYBLOCK_DAY_MILLIS: u64 = SKYBLOCK_MONTH_MILLIS / 31;

                const SKYBLOCK_MONTHS: [&str; 12] = [
                    "Early Spring", "Spring", "Late Spring",
                    "Early Summer", "Summer", "Late Summer",
                    "Early Autumn", "Autumn", "Late Autumn",
                    "Early Winter", "Winter", "Late Winter",
                ];

                let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
                let elapsed = now.saturating_sub(SKYBLOCK_EPOCH_START_MILLIS);
                let day = (elapsed % SKYBLOCK_YEAR_MILLIS) / SKYBLOCK_DAY_MILLIS;
                let month = (day / 31) as usize;
                let day_of_month = (day % 31) + 1;

                let suffix = match day_of_month % 100 {
                    11..=13 => "th",
                    _ => match day_of_month % 10 {
                        1 => "st",
                        2 => "nd",
                        3 => "rd",
                        _ => "th",
                    },
                };
                format!("{} {}{}", SKYBLOCK_MONTHS[month], day_of_month, suffix)
            };

            let room_id = if let Some(room_index) = server.dungeon.get_player_room(player) {
                if room_index < server.dungeon.rooms.len() {
                    let room = &server.dungeon.rooms[room_index];
                    
                    // removed periodic room bounds chat
                    
                    &room.room_data.id
                } else {
                    eprintln!("Warning: Room index {} out of bounds for rooms vector of length {}", room_index, server.dungeon.rooms.len());
                    ""
                }
            } else if server.dungeon.is_player_in_boss_room(player) {
                "bossroom"
            } else {
                ""
            };

            sidebar_lines.push(formatdoc! {r#"
                §e§lSKYBLOCK
                §7{date} §8local {room_id}

                {current_skyblock_month}
                §7{time}
                 §7⏣ §cThe Catacombs §7(F7)

            "#});

            match server.dungeon.state {
                DungeonState::NotReady => {
                    for p in player.server_mut().world.players.values() {
                        sidebar_lines.push(format!("§c[M] §7{}", p.profile.username))
                    }
                    sidebar_lines.new_line();
                }
                DungeonState::Starting { tick_countdown } => {
                    for p in player.server_mut().world.players.values() {
                        sidebar_lines.push(format!("§a[M] §7{}", p.profile.username))
                    }
                    sidebar_lines.new_line();
                    sidebar_lines.push(format!("Starting in: §a0§a:0{}", (tick_countdown / 20) + 1));
                    sidebar_lines.new_line();
                }
                DungeonState::Started { current_ticks } => {
                    // this is scuffed but it works
                    let seconds = current_ticks / 20;
                    let time = if seconds >= 60 {
                        let minutes = seconds / 60;
                        let seconds = seconds % 60;
                        format!(
                            "{}{}m{}{}s",
                            if minutes < 10 { "0" } else { "" },
                            minutes,
                            if seconds < 10 { "0" } else { "" },
                            seconds
                        )
                    } else {
                        let seconds = seconds % 60;
                        format!("{}{}s", if seconds < 10 { "0" } else { "" }, seconds)
                    };
                    // TODO: display correct keys, and cleared percentage
                    // clear percentage is based on amount of tiles that are cleared.
                    sidebar_lines.push(formatdoc! {r#"
                        Keys: §c■ §c✖ §8§8■ §a0x
                        Time elapsed: §a§a{time}
                        Cleared: §c{clear_percent}% §8§8({score})

                        §3§lSolo

                    "#,
                    clear_percent = "0",
                    score = "0",
                    });
                }
                DungeonState::Finished => {}
            }

            if let Some(tab_list) = &tab_list_packet {
                player.write_packet(tab_list);
            }

            sidebar_lines.push_str("§emc.hypixel.net");
            player.sidebar.write_update(sidebar_lines, &mut player.packet_buffer);

            if player.ticks_existed % 60 == 0 {
                player.write_packet(&AddEffect {
                    entity_id: VarInt(player.entity_id),
                    effect_id: 3,
                    amplifier: 2,
                    duration: VarInt(200),
                    hide_particles: true,
                });
                player.write_packet(&AddEffect {
                    entity_id: VarInt(player.entity_id),
                    effect_id: 16,
                    amplifier: 0,
                    duration: VarInt(400),
                    hide_particles: true,
                });
            }
            
            // Apply lava boost system (only in boss rooms)
            let is_in_boss_room = server.dungeon.is_player_in_boss_room(player);
            // We need to check lava in the world, but we can't borrow world while player is mutably borrowed
            // So we'll pass the world reference through the player's world_mut method
            let world_ref = player.world_mut();
            apply_lava_boost(player, world_ref, is_in_boss_room);
            
            player.last_position = player.position;
            player.flush_packets();
        }
        for chunk in &mut server.world.chunk_grid.chunks {
            chunk.packet_buffer = PacketBuffer::new();
        }
        // println!("time elapsed {:?}", start.elapsed());
    }
}

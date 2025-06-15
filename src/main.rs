mod net;
mod server;
mod dungeon;

use crate::dungeon::crushers::Crusher;
use crate::dungeon::room::{Room, RoomType};
use crate::dungeon::Dungeon;
use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::confirm_transaction::ConfirmTransaction;
use crate::net::packets::client_bound::entity::entity_effect::{EntityEffect, HASTEID};
use crate::net::packets::client_bound::particles::Particles;
use crate::net::packets::packet::SendPacket;
use crate::net::run_network::run_network_thread;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::ai::pathfinding::pathfinder::Pathfinder;
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_type::EntityType;
use crate::server::server::Server;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::direction::Direction;
use crate::server::utils::particles::ParticleTypes;
use crate::server::utils::vec3f::Vec3f;
use anyhow::Result;
use std::time::Duration;
use tokio::sync::mpsc::unbounded_channel;

const STATUS_RESPONSE_JSON: &str = r#"{
    "version": { "name": "1.8.9", "protocol": 47 },
    "players": { "max": 1, "online": 0 },
    "description": { "text": "RustClear", "color": "gold", "extra": [{ "text": " version ", "color": "gray" }, { "text": "0.1.0", "color": "green"}] }
}"#;

#[tokio::main]
async fn main() -> Result<()> {
    let (network_tx, network_rx) = unbounded_channel::<NetworkMessage>();
    let (event_tx, mut event_rx) = unbounded_channel::<ClientEvent>();

    let mut server = Server::initialize(network_tx);
    server.world.server = &mut server;

    // for x in 0..100 {
    //     for z in 0..100 {
    //         server.world.chunk_grid.set_block_at(
    //             Blocks::Stone,
    //             x,
    //             0,
    //             z
    //         );
    //     }
    // }

    let spawn_pos = Vec3f {
        x: 6.0,
        y: 1.0,
        z: 6.0,
    };
    // this stuff all being here is kinda messy we should move it at some point.
    let dungeon = Dungeon::with_rooms(vec![
            Room {
                room_x: 0,
                room_z: 0,
                tick_amount: 0,
                room_type: RoomType::Shape2x1,
            },
            Room {
                room_x: 1,
                room_z: 0,
                tick_amount: 0,
                room_type: RoomType::Shape1x1,
            },
            Room {
                room_x: 1,
                room_z: 1,
                tick_amount: 0,
                room_type: RoomType::Shape2x2,
            },
        ]);

    for room in &dungeon.rooms {
        room.load_room(&mut server.world)
    }

    for x in 0..5 {
        for y in 0..5 {
            server.world.set_block_at(Blocks::Stone, x, y, 20);
        }
    }

    // blocks for pathfinding testing
    let blocks = [
        (8, 1, 5),
        (8, 1, 6),
        (8, 1, 7),
        (7, 1, 5),
        (7, 1, 6),
        (7, 1, 7),
        (9, 1, 5),
        (9, 1, 6),
        (9, 1, 7),
        (6, 1, 8),
        (5, 1, 8)
    ];
    for (x, y, z) in blocks.iter() {
        server.world.set_block_at(Blocks::Stone, *x, *y, *z);
    }

    let mut crusher = Crusher::new(
        BlockPos {
            x: 20,
            y: 1,
            z: 20,
        },
        Direction::North,
        5,
        5,
        10,
        10,
        20,
    );

    let mut tick_interval = tokio::time::interval(Duration::from_millis(50));
    tokio::spawn(
        run_network_thread(
            network_rx,
            server.network_tx.clone(),
            event_tx.clone()
        )
    );

    let zombie = Entity::create_at(EntityType::Zombie, spawn_pos, server.world.new_entity_id());
    let path = Pathfinder::find_path(&zombie, &BlockPos { x: 10, y: 1, z: 10 }, &server.world)?;

    server.world.entities.insert(zombie.entity_id, zombie);
    let text = ChatComponentTextBuilder::new("Hello World!").build();
    server.world.player_info.update_text(1, text);

    loop {
        tick_interval.tick().await;

        while let Ok(message) = event_rx.try_recv() {
            server.process_event(message).unwrap_or_else(|err| eprintln!("Error processing event: {err}"));
        }

        for entity_id in server.world.entities.keys().cloned().collect::<Vec<_>>() {
            if let Some(mut entity) = server.world.entities.remove(&entity_id) {
                entity.ticks_existed += 1;
                // this may at some point be abused to prevent getting an entities own self if it iterates over world entities so be careful if you change this
                let returned = entity.update(&mut server.world, &server.network_tx);
                server.world.entities.insert(entity_id, returned);
            }
        }

        // this needs to be changed to work with loaded chunks, tracking last sent data per player (maybe), etc.
        // also needs to actually be in a vanilla adjacent way.
        for player in server.players.values_mut() {
            // println!("player ticked: {player:?}");
            ConfirmTransaction::new().send_packet(player.client_id, &server.network_tx)?; // should stop disconnects? keep alive logic would too probably.
            // for entity in player.tracked_entities.iter() {
            //     if let Some(entity) = server.world.entities.get_mut(entity) {
            //         EntityLookMove::from_entity(entity).send_packet(player.client_id, &server.network_tx)?;
            //         EntityHeadLook::new(entity.entity_id, entity.head_yaw).send_packet(player.client_id, &server.network_tx)?;
            //     }
            // }

            if player.scoreboard.header_dirty {
                player.scoreboard.header_packet().send_packet(player.client_id, &server.network_tx)?;
            }

            // maybe another value if any lines are updated? this will just not pull any packets if nothing is updated but it will still iterate...
            for packet in player.scoreboard.get_packets() {
                packet.send_packet(player.client_id, &server.network_tx)?;
            }

            if !player.scoreboard.displaying {
                player.scoreboard.display_packet().send_packet(player.client_id, &server.network_tx)?;
            }

            if let Some(player_entity) = server.world.entities.get(&player.entity_id) {
                if player_entity.ticks_existed % 20 == 0 {
                    let seconds = player_entity.ticks_existed / 20;
                    player.scoreboard.update_line("etime", format!("Time Elapsed: §a§a{seconds}s")); // this isnt accurate to hypixel atm but its ok!
                }

                if player_entity.ticks_existed % 150 == 0 {
                    //player.scoreboard.add_line_at(0, "resize", "amazing");

                    // player.scoreboard.update_header("NEW HEADER WOWOWOW");
                }

                if player_entity.ticks_existed % 250 == 0 {
                    player.scoreboard.remove_line("etime");

                    // player.scoreboard.update_header("old header :(");
                }

                if player_entity.ticks_existed % 5 == 0 {
                    let mut current_index = 1;
                    for pos in path.iter() {
                        let particle = Particles::new(
                            ParticleTypes::Crit,
                            Vec3f::from(pos),
                            Vec3f::new(0.1, 0.1, 0.1),
                            0.0,
                            current_index,
                            true,
                            None,
                        );
                        current_index += 1;

                        particle?.send_packet(player.client_id, &server.network_tx)?;
                    }
                }

                if player_entity.ticks_existed % 60 == 0 {
                    EntityEffect {
                        entity_id: player.entity_id,
                        effect_id: HASTEID,
                        amplifier: 2,
                        duration: 200,
                        hide_particles: true,
                    }.send_packet(player.client_id, &server.network_tx)?;

                    // EntityEffect {
                    //     entity_id: player.entity_id,
                    //     effect_id: NIGHTVISIONID,
                    //     amplifier: 0,
                    //     duration: 400,
                    //     hide_particles: true,
                    // }.send_packet(player.client_id, &server.network_tx)?;
                }
            }

            dungeon.get_room(player);
        }

        // if  {  }

        crusher.tick(&mut server);
    }
}
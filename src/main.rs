mod net;
mod server;
mod dungeon;

use crate::dungeon::crushers::Crusher;
use crate::dungeon::Dungeon;
use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::confirm_transaction::ConfirmTransaction;
use crate::net::packets::packet::SendPacket;
use crate::net::run_network::run_network_thread;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_type::EntityType;
use crate::server::server::Server;
use crate::server::utils::direction::Direction;
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

    let spawn_pos = Vec3f {
        x: 25.0,
        y: 69.0,
        z: 25.0,
    };

    let zombie = Entity::create_at(EntityType::Zombie, spawn_pos, server.world.new_entity_id());
    server.world.entities.insert(zombie.entity_id, zombie);

    let dungeon = Dungeon::from_string("040809090104050409091011121314151516121314041516121714031802120414061818009999999309099109099199090999999909099999910092999999190099");

    for room in &dungeon.rooms {
        dungeon.load_room(room, &mut server.world);
    }

    for door in &dungeon.doors {
        dungeon.load_door(door, &mut server.world);
    }

    let mut crusher = Crusher::new(
        BlockPos {
            x: 30,
            y: 69,
            z: 20,
        },
        Direction::North,
        5,
        5,
        10,
        10,
        20,
    );

    let fairy_room = include_bytes!("room_data/Fairy_462_-312_1x1");

    println!("{} {}", fairy_room.len(), fairy_room.len() / 2);

    for i in (0..fairy_room.len() - 1).step_by(2) {
        let state_id = ((fairy_room[i] as u16) << 8) | fairy_room[i+1] as u16;

        // println!("{:#b} | {} | {}", state_id, state_id >> 4, state_id & 0xF);

        let block = Blocks::from_block_state_id(state_id);

        // println!("{:?}", block);
        let num = i / 2;
        let x = num % 31;
        let z = (num / 31) % 31;
        let y = num / (31 * 31);

        server.world.set_block_at(block, x as i32, y as i32, z as i32);

    }

    // println!("{:?}", fairy_room);

    let mut tick_interval = tokio::time::interval(Duration::from_millis(50));
    tokio::spawn(
        run_network_thread(
            network_rx,
            server.network_tx.clone(),
            event_tx.clone()
        )
    );

    loop {
        tick_interval.tick().await;

        while let Ok(message) = event_rx.try_recv() {
            server.process_event(message).unwrap_or_else(|err| eprintln!("Error processing event: {err}"));
        }

        for entity_id in server.world.entities.keys().cloned().collect::<Vec<_>>() {
            if let Some(entity) = server.world.entities.remove(&entity_id) {
                // this may at some point be abused to prevent getting an entities own self if it iterates over world entities so be careful if you change this
                let returned = entity.update(&mut server.world, &server.network_tx);
                server.world.entities.insert(entity_id, returned);
            }
        }

        // this needs to be changed to work with loaded chunks, tracking last sent data per player (maybe), etc.
        // also needs to add a method to only send the right entity packet given movement data based on last sent.
        // also needs to actually be in a vanilla adjacent way.
        for player in server.players.values() {
            // println!("player ticked: {player:?}");
            ConfirmTransaction::new().send_packet(player.client_id, &server.network_tx)?; // should stop disconnects? keep alive logic would too probably.
            // for entity in player.tracked_entities.iter() {
            //     if let Some(entity) = server.world.entities.get_mut(entity) {
            //         EntityLookMove::from_entity(entity).send_packet(player.client_id, &server.network_tx)?;
            //         EntityHeadLook::new(entity.entity_id, entity.head_yaw).send_packet(player.client_id, &server.network_tx)?;
            //     }
            // }

            let room = dungeon.get_player_room(player);

        }

        // if  {  }

        crusher.tick(&mut server);
    }
}
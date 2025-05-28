use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::chunk_data::ChunkData;
use crate::net::packets::client_bound::join_game::JoinGame;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::client_bound::spawn_mob::SpawnMob;
use crate::net::packets::packet::{SendPacket, ServerBoundPacket};
use crate::net::packets::packet_registry::ClientBoundPacket;
use crate::server::block::Blocks;
use crate::server::chunk::chunk_section::ChunkSection;
use crate::server::chunk::Chunk;
use crate::server::entity::entity_enum::{EntityEnum, EntityTrait};
use crate::server::entity::player_entity::PlayerEntity;
use crate::server::entity::zombie::Zombie;
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub async fn tick(mut event_rx: UnboundedReceiver<ClientEvent>, network_tx: UnboundedSender<NetworkMessage>) -> anyhow::Result<()> {
    let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(50));
    
    let mut world = World::with_net_tx(network_tx);

    loop {
        tick_interval.tick().await;

        // Handle incoming events from network
        while let Ok(event) = event_rx.try_recv() {
            match event {
                ClientEvent::PacketReceived { client_id, packet } => {
                    //println!("Client {} sent {:?}", client_id, packet);

                    packet.main_process(&mut world, client_id).unwrap_or_else(|e| {
                        println!("Error processing packet: {:?}", e);
                    });
                    
                    match packet {
                        _ => {}
                    }
                }
                ClientEvent::NewPlayer { client_id } => {
                    let player = PlayerEntity::spawn_at(world.world_spawn.clone() + Vec3f::from_y(10.0), client_id, &mut world);

                    // this stuff should be moved into the player entity and handled there.

                    ClientBoundPacket::from(JoinGame::from_player(&player)).send_packet(client_id, &world.network_tx)?;

                    // spawn in sky for now
                    let spawn_position_look = PositionLook {
                        x: player.entity.pos.x,
                        y: player.entity.pos.y,
                        z: player.entity.pos.z,
                        yaw: player.entity.yaw,
                        pitch: player.entity.pitch,
                        flags: 0,
                    };

                    ClientBoundPacket::from(spawn_position_look).send_packet(client_id, &world.network_tx)?;

                    let mut chunk_section = ChunkSection::new();
                    for x in 0..16 {
                        for z in 0..16 {
                            chunk_section.set_block_at(Blocks::Stone, x, 0, z)
                        }
                    }
                    
                    let mut chunk = Chunk::new(0, 0);
                    chunk.add_section(chunk_section, 0);

                    ChunkData::from_chunk(&chunk, true).send_packet(client_id, &world.network_tx)?;

                    world.spawn_entity(EntityEnum::from(player));

                    let spawn_vec = world.world_spawn.clone() + Vec3f { x: 5.0, y: 1.0, z: 5.0 };

                    let mut zombie = Zombie::create_at(spawn_vec, &mut world);

                    SpawnMob::from_entity(&mut zombie).send_packet(client_id, &world.network_tx)?;

                    world.spawn_entity(EntityEnum::from(zombie));

                    //world.add_entity(PlayerEntity(player));
                }
                ClientEvent::ClientDisconnected { client_id } => {
                    let _ = world.remove_player_from_client_id(&client_id);
                    println!("Client {} disconnected", client_id);
                }
            }
        }

        if world.current_server_tick % 20 == 0 {
            // world time update packet probably
        }

        // Game logic here...
    }
}
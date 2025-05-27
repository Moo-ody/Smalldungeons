use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::{chunk_data, join_game, position_look};
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_registry::ClientBoundPackets::{ChunkData, JoinGame, PositionLook};
use crate::server::block::Blocks;
use crate::server::chunk::chunk_section::ChunkSection;
use crate::server::chunk::Chunk;
use crate::server::entity::entity_enum::{EntityEnum, EntityTrait};
use crate::server::entity::player_entity::PlayerEntity;
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};


pub async fn tick(mut event_rx: UnboundedReceiver<ClientEvent>, network_tx: UnboundedSender<NetworkMessage>) -> anyhow::Result<()> {
    let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(50));
    
    let mut world = World::with_net_tx(network_tx);
    let mut current_tick: u64 = 0;

    loop {
        tick_interval.tick().await;
        current_tick += 1;
        world.current_server_tick = current_tick;

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
                    let player = PlayerEntity::spawn_at(Vec3f::new_empty(), client_id, &mut world);

                    JoinGame(join_game::JoinGame::from_player(&player)).send_packet(client_id, &world.network_tx)?;

                    // spawn in sky for now
                    let packet = position_look::PositionLook {
                        x: player.entity.pos.x,
                        y: player.entity.pos.y + 10.0,
                        z: player.entity.pos.z,
                        yaw: player.entity.yaw,
                        pitch: player.entity.pitch,
                        flags: 0,
                    };

                    PositionLook(packet).send_packet(client_id, &world.network_tx)?;

                    let mut chunk_section = ChunkSection::new();
                    for x in 0..16 {
                        for z in 0..16 {
                            chunk_section.set_block_at(Blocks::Stone, x, 0, z)
                        }
                    }
                    
                    let mut chunk = Chunk::new(0, 0);
                    chunk.add_section(chunk_section, 0);
                    
                    ChunkData(
                        chunk_data::ChunkData::from_chunk(&chunk, true)
                    ).send_packet(client_id, &world.network_tx)?;

                    world.spawn_entity(EntityEnum::from(player))
                    
                    //world.add_entity(PlayerEntity(player));
                }
                ClientEvent::ClientDisconnected { client_id } => {
                    world.remove_player_from_client_id(&client_id);
                    println!("Client {} disconnected", client_id);
                }
            }
        }

        world.tick();
        
        if current_tick % 20 == 0 {
            // world time update packet probably
        }

        // Game logic here...
    }
}
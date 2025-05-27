use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::MissedTickBehavior;
use crate::net::client_event::ClientEvent;
use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::{chunk_data, join_game, position_look, keep_alive};
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_registry::ClientBoundPackets::{ChunkData, JoinGame, PositionLook};
use crate::server::entity::entity_enum::{EntityEnum, EntityTrait};
use crate::server::entity::player_entity::PlayerEntity;
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;


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
                    PositionLook(position_look::PositionLook::from_player(&player)).send_packet(client_id, &world.network_tx)?;
                    ChunkData(chunk_data::ChunkData::new()).send_packet(client_id, &world.network_tx)?;

                    world.spawn_entity(EntityEnum::from(player))
                    
                    //world.add_entity(PlayerEntity(player));
                }
                ClientEvent::ClientDisconnected { client_id } => {
                    world.remove_player_from_client_id(&client_id);
                    println!("Client {} disconnected", client_id);
                    // this is getting sent twice somewhere on kick
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
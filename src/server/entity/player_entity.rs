use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::confirm_transaction;
use crate::net::packets::client_bound::disconnect;
use crate::net::packets::client_bound::keep_alive::KeepAlive;
use crate::net::packets::packet_registry::ClientBoundPackets::{CBConfirmTransaction, CBKeepAlive, Disconnect};
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_enum::EntityTrait;
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct PlayerEntity {
    pub client_id: u32,
    pub last_keep_alive: i32,
    pub ping: i32,
    pub entity: Entity,
}

impl PlayerEntity {
    pub fn spawn_at(pos: Vec3f, id: u32, world: &mut World) -> PlayerEntity {
        let entity_id = world.new_entity_id();
        world.client_to_entities.insert(id, entity_id);
        PlayerEntity {
            client_id: id,
            last_keep_alive: 0,
            ping: 0,
            entity: Entity::spawn_at(pos, entity_id)
        }
    }
    
    pub fn disconnect(&mut self, world: &mut World, reason: &str) {
        world.network_tx.send(NetworkMessage::SendPacket {
            client_id: self.client_id,
            packet: Disconnect(disconnect::Disconnect {
                reason: format!("{{\"text\":\"{}\"}}", reason),
            }),
        }).unwrap_or_else(|e| eprintln!("Error sending disconnect packet: {:?}", e));
        
        world.network_tx.send(NetworkMessage::DisconnectClient {
            client_id: self.client_id,
        }).unwrap_or_else(|e| eprintln!("Error disconnecting client: {:?}", e))
    }
}

impl EntityTrait for PlayerEntity {
    
    fn get_entity(&mut self) -> &mut Entity {
        &mut self.entity
    }

    fn tick(&mut self, world: &mut World) -> anyhow::Result<()> {
        if self.client_id != 0 {
            CBConfirmTransaction(confirm_transaction::ConfirmTransaction::new()).send_packet(self.client_id, &world.network_tx)?;
            
            if world.current_server_tick % 50 == 0 {
                let time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i32;
                self.last_keep_alive = time;
                CBKeepAlive(KeepAlive::from_time(time)).send_packet(self.client_id, &world.network_tx)?;
            } // this hsould be entirely handled by network thread instead i think maybe.
            
            // if self.entity.ticks_existed >= 100 {
            //     self.disconnect(world, "go away nothing shere yet");
            // }
            // 
            
        }
        
        Ok(())
    }

    fn spawn(&mut self)  {
        // todo
    }

    fn despawn(&mut self, world: &mut World) {
        // todo
    }
}
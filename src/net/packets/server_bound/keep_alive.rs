use std::time::{SystemTime, UNIX_EPOCH};
use crate::net::packets::packet::ServerBoundPacket;
use crate::net::packets::packet_context::PacketContext;
use bytes::BytesMut;
use crate::net::varint::read_varint;
use crate::server::entity::entity_enum::EntityEnum::PlayerEntity;
use crate::server::entity::entity_enum::EntityTrait;
use crate::server::world::World;

#[derive(Debug)]
pub struct KeepAlive {
    pub id: i32,
}

#[async_trait::async_trait]
impl ServerBoundPacket for KeepAlive {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        let id = read_varint(buf).ok_or_else(|| anyhow::anyhow!("Failed to read keep alive id"))?;
        Ok(KeepAlive {
            id
        })
    }

    async fn process(&self, context: PacketContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn main_process(&self, world: &mut World, client_id: u32) -> anyhow::Result<()> {
        let player = world.get_player_from_client_id(client_id).ok_or_else(|| anyhow::anyhow!("Couldnt get player"))?;
        
        if let PlayerEntity(player_entity) = player {
            if (player_entity.last_keep_alive == self.id) {
                let since = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i32 - player_entity.last_keep_alive;
                player_entity.ping = (player_entity.ping * 3 + since) / 4;
            }
            println!("Ping: {}", player_entity.ping);
        }
        
        Ok(())
    }
}
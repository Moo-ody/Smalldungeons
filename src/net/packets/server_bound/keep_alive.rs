use crate::net::packets::packet::ServerBoundPacket;
use crate::net::var_int::read_var_int;
use crate::server::player::Player;
use crate::server::world::World;
use anyhow::Context;
use bytes::BytesMut;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct KeepAlive {
    pub id: i32,
}

#[async_trait::async_trait]
impl ServerBoundPacket for KeepAlive {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        let id = read_var_int(buf).context("Failed to read keep alive id")?;
        Ok(KeepAlive {
            id
        })
    }

    fn main_process(&self, _: &mut World, player: &mut Player) -> anyhow::Result<()> {
        if player.last_keep_alive == self.id {
            let since = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i32 - player.last_keep_alive;
            player.ping = (player.ping * 3 + since) / 4;
            println!("Ping: {}", player.ping);
        }
        Ok(())
    }
}
use crate::net::packets::old_packet::ServerBoundPacket;
use crate::net::packets::server_bound::client_status::ClientStatus::{OpenInventory, PerformRespawn, RequestStats};
use crate::net::var_int::read_var_int;
use crate::server::player::player::Player;
use crate::server::world::World;
use bytes::BytesMut;

#[derive(Debug)]
pub enum ClientStatus {
    PerformRespawn,
    RequestStats,
    OpenInventory,
}

#[derive(Debug)]
pub struct ClientStatusPacket {
    status: ClientStatus,
}

#[async_trait::async_trait]
impl ServerBoundPacket for ClientStatusPacket {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(ClientStatusPacket {
            status: match read_var_int(buf).unwrap() { 
                0 => PerformRespawn,
                1 => RequestStats,
                _ => OpenInventory,
            },
        })
    }

    fn main_process(&self, _: &mut World, player: &mut Player) -> anyhow::Result<()> {
        match self.status {
            PerformRespawn => { /* maybe needed, not sure */ }
            RequestStats => { /* not needed at all */ }
            // TOODO:
            OpenInventory => {} /*player.open_ui(UI::Inventory)?*/
        }
        Ok(())
    }
}
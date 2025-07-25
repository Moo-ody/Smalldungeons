use crate::net::packets::old_packet::ServerBoundPacket;
use crate::net::var_int::read_var_int;
use crate::server::player::player::Player;
use crate::server::world::World;
use bytes::BytesMut;

#[derive(Debug)]
pub struct PlayerAction {
    pub entity_id: i32,
    pub action: Action,
    pub aux_data: i32 // this is used for stuff like riding jump
}

#[repr(i32)]
#[derive(Debug)]
pub enum Action {
    StartSneaking,
    StopSneaking,
    StopSleeping,
    StartSprinting,
    StopSprinting,
    RidingJump,
    OpenInventory,
}

#[async_trait::async_trait]
impl ServerBoundPacket for PlayerAction {
    async fn read_from(buf: &mut BytesMut) -> anyhow::Result<Self> {
        Ok(PlayerAction {
            entity_id: read_var_int(buf).unwrap(),
            action: {
                match read_var_int(buf).unwrap() {
                    0 => Action::StartSneaking,
                    1 => Action::StopSneaking,
                    2 => Action::StopSleeping,
                    3 => Action::StartSprinting,
                    4 => Action::StopSprinting,
                    5 => Action::RidingJump,
                    _ => Action::OpenInventory,
                }
            },
            aux_data: read_var_int(buf).unwrap(),
        })
    }

    fn main_process(&self, _: &mut World, player: &mut Player) -> anyhow::Result<()> {
        match self.action {
            Action::StartSneaking => player.is_sneaking = true,
            Action::StopSneaking => player.is_sneaking = false,
            _ => {}
        }
        Ok(())
    }
}
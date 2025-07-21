use crate::dungeon::dungeon_state::DungeonState;
use crate::server::block::block_pos::BlockPos;
use crate::server::player::player::Player;

pub enum BlockInteractAction {
    WitherDoor {
        door_index: usize
    },
    BloodDoor {
        door_index: usize
    },
    Callback(fn(&Player, &BlockPos)),
}


impl BlockInteractAction {
    pub fn interact(&self, player: &Player, block_pos: &BlockPos) {
        match self {
            Self::WitherDoor { door_index: id } |
            Self::BloodDoor { door_index: id } => {

                let world = &mut player.server_mut().world;
                let dungeon = &mut player.server_mut().dungeon;

                if let DungeonState::Started { .. } = dungeon.state {
                    // todo check if player has a key
                    let door = &dungeon.doors[*id];
                    door.open_door(world);
                }
            }

            Self::Callback(func) => {
                func(player, block_pos);
            }
        }
    }
}
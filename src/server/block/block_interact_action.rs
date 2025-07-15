use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::OpenDoorTask;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::player::Player;
// not sure if it is fitting here.

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
            BlockInteractAction::WitherDoor { door_index: id } |
            BlockInteractAction::BloodDoor { door_index: id } => {

                let world = &mut player.server_mut().world;
                let dungeon = &mut player.server_mut().dungeon;

                if let DungeonState::Started { .. } = dungeon.state {
                    // todo keys
                    let door = &dungeon.doors[*id];
                    world.fill_blocks(
                        Blocks::Barrier,
                        (door.x - 1, 69, door.z - 1),
                        (door.x + 1, 72, door.z + 1)
                    );
                    dungeon.test.push(OpenDoorTask {
                        ticks_left: 20,
                        door_index: *id,
                    });
                    player.send_msg("clicked door").unwrap();
                }
            }

            BlockInteractAction::Callback(func) => {
                func(player, block_pos);
            }
        }
    }
}
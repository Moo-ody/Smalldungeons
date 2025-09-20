use crate::net::protocol::play::clientbound::{EntityVelocity};
use crate::net::var_int::VarInt;
use crate::server::block::blocks::Blocks;
use crate::server::player::player::Player;
use crate::server::world::World;

// Constants for the lava boost system
const IS_SINGLEPLAYER: bool = true;          // Singleplayer detection
// const GROUND_EPSILON: f64 = 0.5;             // "close to ground" tolerance - not used anymore
const BOOST_VELOCITY_Y: f64 = 3.5;           // vertical launch power

/// Check if a player is touching lava (either standing on it or in it)
fn is_player_touching_lava(player: &Player, world: &World) -> bool {
    let block_x = player.position.x.floor() as i32;
    let block_z = player.position.z.floor() as i32;
    
    // Check block at player's feet (below their position)
    let feet_y = (player.position.y - 0.1).floor() as i32;
    let block_below = world.get_block_at(block_x, feet_y, block_z);
    let standing_on_lava = matches!(block_below, Blocks::Lava { .. } | Blocks::FlowingLava { .. });
    
    // Check block at player's position (in lava)
    let player_y = player.position.y.floor() as i32;
    let block_at_player = world.get_block_at(block_x, player_y, block_z);
    let in_lava = matches!(block_at_player, Blocks::Lava { .. } | Blocks::FlowingLava { .. });
    
    standing_on_lava || in_lava
}

// Ground proximity check removed - touching lava should always bounce

/// Apply lava boost to a player if they meet all conditions
/// This function should be called every tick for each player
pub fn apply_lava_boost(player: &mut Player, world: &World, is_in_boss_room: bool) {
    // 1) Singleplayer gate
    if !IS_SINGLEPLAYER {
        return;
    }
    
    // 2) Must be in boss room
    if !is_in_boss_room {
        return;
    }
    
    // 3) Are our feet touching lava?
    if !is_player_touching_lava(player, world) {
        return;
    }
    
    // 4) Are we basically on the ground? (within 0.5 above the integer Y)
    // Note: Removed this check since touching lava should always bounce
    // if !is_near_ground(player) {
    //     return;
    // }
    
    // 5) Apply boost: set player velocity upward like the Java version
    // This creates a proper bounce effect instead of just teleporting
    player.write_packet(&EntityVelocity {
        entity_id: VarInt(player.entity_id),
        velocity_x: 0, // Keep current horizontal velocity
        velocity_y: (BOOST_VELOCITY_Y * 8000.0) as i16, // Convert to packet format
        velocity_z: 0, // Keep current horizontal velocity
    });
}

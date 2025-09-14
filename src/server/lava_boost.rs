use crate::net::protocol::play::clientbound::{EntityVelocity, PositionLook};
use crate::net::var_int::VarInt;
use crate::server::block::blocks::Blocks;
use crate::server::player::player::Player;
use crate::server::world::World;

// Constants for the lava boost system
const IS_SINGLEPLAYER: bool = true;          // Singleplayer detection
const GROUND_EPSILON: f64 = 0.10;            // "close to ground" tolerance
const BOOST_VELOCITY_Y: f64 = 3.5;           // vertical launch power

/// Check if a player is currently in lava by checking the block at their position
fn is_player_in_lava(player: &Player, world: &World) -> bool {
    let block_x = player.position.x.floor() as i32;
    let block_y = player.position.y.floor() as i32;
    let block_z = player.position.z.floor() as i32;
    
    let block = world.get_block_at(block_x, block_y, block_z);
    
    matches!(block, Blocks::Lava { .. } | Blocks::FlowingLava { .. })
}

/// Check if a player is close to the ground (within epsilon of integer Y)
fn is_near_ground(player: &Player) -> bool {
    let y_int = player.position.y.floor();
    (player.position.y - y_int) < GROUND_EPSILON
}

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
    
    // 3) Are we in lava?
    if !is_player_in_lava(player, world) {
        return;
    }
    
    // 4) Are we basically on the ground? (within 0.1 above the integer Y)
    if !is_near_ground(player) {
        return;
    }
    
    // 5) Apply boost: set player velocity upward like the Java version
    // This creates a proper bounce effect instead of just teleporting
    player.write_packet(&EntityVelocity {
        entity_id: VarInt(player.entity_id),
        velocity_x: 0, // Keep current horizontal velocity
        velocity_y: (BOOST_VELOCITY_Y * 8000.0) as i16, // Convert to packet format
        velocity_z: 0, // Keep current horizontal velocity
    });
}

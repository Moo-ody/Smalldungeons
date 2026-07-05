//! Global green-room disengagement: if a mob's target has walked into the entrance room,
//! the mob gives up the chase - implemented once here rather than duplicated per mob.
//!
//! "Green room" is not a guess: `dungeon/map.rs` itself comments
//! `"Skip checkmark for entrance room (green room) and fairy room (pink room)"` - the
//! entrance room is the one this codebase already calls the green room (the fairy room is
//! the *pink* room, and is not included).

use crate::dungeon::room::room_data::RoomType;
use crate::server::player::player::ClientId;
use crate::server::world::World;

pub fn is_green_room(world: &World, room_index: usize) -> bool {
    world.server_mut().dungeon.rooms.get(room_index)
        .is_some_and(|room| room.room_data.room_type == RoomType::Entrance)
}

/// Whether `target`'s current room (if known) is the green room.
pub fn target_in_green_room(world: &World, target: ClientId) -> bool {
    world.players.get(&target)
        .and_then(|player| player.current_room_index)
        .is_some_and(|room_index| is_green_room(world, room_index))
}

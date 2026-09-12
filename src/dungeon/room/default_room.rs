//! "Default" (a Champion/"yellow" room, real name "Default", red-carpet decor) always spawns
//! one Lost Adventurer, per explicit request - regardless of which of this room's captured
//! mob-data layouts (`room_data/mobs/default.json`) happens to load. Only one of its captured
//! `runIndex` groups actually includes a Lost Adventurer, so `spawner::spawn_room_mobs`'
//! random-layout pick alone can't guarantee one; this is an ad-hoc spawn layered on top, same
//! shape as `spawner::spawn_shadow_assassin`/`spawn_king_midas`. Spawned once on room entry,
//! same timing as every other room-entry-hooked mob here (Shadow Assassin, Water Board's gate
//! roll, Three Weirdos).
//!
//! There are actually *two* different physical rooms both named "Default" in the room_data pool
//! (`room_data/rooms/68,default,210,66.json` and `.../68,default,246,102.json`) - confirmed by
//! explicit follow-up report, not the same design captured twice as originally assumed. Checking
//! `room_data.name` alone would fire this for both, so `room_data.id` (its own coordinate string,
//! e.g. "246,102") disambiguates: the id checked here is the one with real red wool carpet
//! (dyed-color metadata 14) in its own `block_data` - confirmed by counting block metadata
//! directly rather than guessed - the other ("210,66", lots of dirt) gets its own separate spawn,
//! see `default_dirt::setup`.
//!
//! Position is a literal absolute Y (matches the `BlockPos`/crypts convention) - this room's
//! `id` pins it to exactly one real capture (`bottom: 58`), so unlike before there's no second
//! layout with a different `bottom` to worry about landing underground in.

use crate::dungeon::room::room::Room;
use crate::server::block::block_position::BlockPos;
use crate::server::block::rotatable::Rotatable;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;

/// The real red-carpet "Default" capture's own id - see the module doc comment.
const ROOM_ID: &str = "246,102";

/// Room-relative (pre-rotation) spawn position, given directly.
const SPAWN_POSITION: BlockPos = BlockPos { x: 12, y: 63, z: 22 };

/// Sets up "Default"'s guaranteed Lost Adventurer if `room` actually is that specific capture.
/// No-op for every other room (belt-and-suspenders check; the caller already filters by name and
/// by `Room::default_lost_adventurer_spawned` before calling this at all).
pub fn setup(room: &Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Default" || room.room_data.id != ROOM_ID {
        return;
    }

    let world_pos = room.get_world_block_pos(&SPAWN_POSITION);
    let position = DVec3::new(world_pos.x as f64 + 0.5, world_pos.y as f64, world_pos.z as f64 + 0.5);
    let yaw = 180.0_f32.rotate(room.rotation);

    crate::server::entity::dungeon_mobs::spawner::spawn_lost_adventurer(world, room_index, room.entered, position, yaw);
}

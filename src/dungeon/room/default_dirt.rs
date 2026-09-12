//! The *other* "Default" room (real name "Default", lots of dirt - see `default_room.rs`'s own
//! doc comment for how the two same-named "Default" captures are told apart via `room_data.id`)
//! always spawns exactly one miniboss, randomly one of Shadow Assassin, Lost Adventurer, Frozen
//! Adventurer, or Angry Archaeologist - per explicit request. Same ad-hoc, room-entry-hooked
//! shape as `dragon.rs` (which this is otherwise a near-duplicate of - kept as its own file
//! rather than folded into `default_room.rs`, since that module's whole job is the *other*
//! "Default" capture specifically).
//!
//! The roll uses the dungeon's seeded RNG (`seeded_rng`), same convention as `dragon.rs`'s own
//! roll - so which miniboss shows up is fixed for the whole dungeon, not re-rolled on a room
//! re-entry (guarded by `Room::default_dirt_miniboss_spawned` regardless).

use crate::dungeon::room::room::Room;
use crate::server::block::block_position::BlockPos;
use crate::server::block::rotatable::Rotatable;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use crate::utils::seeded_rng::seeded_rng;
use rand::seq::IndexedRandom;

/// The real "lots of dirt" Default capture's own id - see the module doc comment and
/// `default_room.rs`'s own for how this was told apart from the red-carpet one.
const ROOM_ID: &str = "210,66";

/// Room-relative (pre-rotation) spawn position, given directly.
const SPAWN_POSITION: BlockPos = BlockPos { x: 13, y: 70, z: 5 };

#[derive(Clone, Copy)]
enum Miniboss {
    ShadowAssassin,
    LostAdventurer,
    FrozenAdventurer,
    AngryArchaeologist,
}

const MINIBOSSES: [Miniboss; 4] = [
    Miniboss::ShadowAssassin,
    Miniboss::LostAdventurer,
    Miniboss::FrozenAdventurer,
    Miniboss::AngryArchaeologist,
];

/// Sets up this "Default" capture's random miniboss if `room` actually is that specific one.
/// No-op for every other room (belt-and-suspenders check; the caller already filters by name and
/// by `Room::default_dirt_miniboss_spawned` before calling this at all).
pub fn setup(room: &Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Default" || room.room_data.id != ROOM_ID {
        return;
    }

    let world_pos = room.get_world_block_pos(&SPAWN_POSITION);
    let position = DVec3::new(world_pos.x as f64 + 0.5, world_pos.y as f64, world_pos.z as f64 + 0.5);
    let yaw = 180.0_f32.rotate(room.rotation);

    let &chosen = MINIBOSSES.choose(&mut seeded_rng()).expect("MINIBOSSES is non-empty");
    match chosen {
        Miniboss::ShadowAssassin => {
            crate::server::entity::dungeon_mobs::spawner::spawn_shadow_assassin(world, room_index, room.entered, position, yaw);
        }
        Miniboss::LostAdventurer => {
            crate::server::entity::dungeon_mobs::spawner::spawn_lost_adventurer(world, room_index, room.entered, position, yaw);
        }
        Miniboss::FrozenAdventurer => {
            crate::server::entity::dungeon_mobs::spawner::spawn_frozen_adventurer(world, room_index, room.entered, position, yaw);
        }
        Miniboss::AngryArchaeologist => {
            crate::server::entity::dungeon_mobs::spawner::spawn_angry_archaeologist(world, room_index, room.entered, position, yaw);
        }
    }
}

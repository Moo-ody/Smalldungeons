//! "Dragon" (a Champion/"yellow" room) always spawns exactly one miniboss, randomly one of
//! Shadow Assassin, Lost Adventurer, Frozen Adventurer, or Angry Archaeologist - per explicit
//! request. Same ad-hoc, room-entry-hooked shape as `shadow_assassin`/`default_room` (no
//! captured room-JSON spawn data drives this; the position/roll are given directly).
//!
//! The roll uses the dungeon's seeded RNG (`seeded_rng`), same convention as every other
//! per-dungeon "which variant" pick (room/layout selection, Water Board's pattern, Boulder's
//! layout, etc. - see the session's own RNG-seeding pass) - so which miniboss shows up is fixed
//! for the whole dungeon, not re-rolled on a room re-entry (guarded by
//! `Room::dragon_miniboss_spawned` regardless).

use crate::dungeon::room::room::Room;
use crate::server::block::block_position::BlockPos;
use crate::server::block::rotatable::Rotatable;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use crate::utils::seeded_rng::seeded_rng;
use rand::seq::IndexedRandom;

/// Room-relative (pre-rotation) spawn position, given directly - only one real capture of this
/// room exists (`room_data/rooms/35,dragon,174,66.json`), so unlike "Default"'s two differently-
/// positioned captures, this can just be a literal absolute Y (matching the `BlockPos`/crypts
/// convention) with no bottom-relative conversion needed.
const SPAWN_POSITION: BlockPos = BlockPos { x: 15, y: 69, z: 11 };

#[derive(Clone, Copy)]
enum DragonMiniboss {
    ShadowAssassin,
    LostAdventurer,
    FrozenAdventurer,
    AngryArchaeologist,
}

const MINIBOSSES: [DragonMiniboss; 4] = [
    DragonMiniboss::ShadowAssassin,
    DragonMiniboss::LostAdventurer,
    DragonMiniboss::FrozenAdventurer,
    DragonMiniboss::AngryArchaeologist,
];

/// Sets up "Dragon"'s random miniboss if `room` actually is that room. No-op for every other
/// room (belt-and-suspenders check; the caller already filters by name and by
/// `Room::dragon_miniboss_spawned` before calling this at all).
pub fn setup(room: &Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Dragon" {
        return;
    }

    let world_pos = room.get_world_block_pos(&SPAWN_POSITION);
    let position = DVec3::new(world_pos.x as f64 + 0.5, world_pos.y as f64, world_pos.z as f64 + 0.5);
    let yaw = 180.0_f32.rotate(room.rotation);

    let &chosen = MINIBOSSES.choose(&mut seeded_rng()).expect("MINIBOSSES is non-empty");
    match chosen {
        DragonMiniboss::ShadowAssassin => {
            crate::server::entity::dungeon_mobs::spawner::spawn_shadow_assassin(world, room_index, room.entered, position, yaw);
        }
        DragonMiniboss::LostAdventurer => {
            crate::server::entity::dungeon_mobs::spawner::spawn_lost_adventurer(world, room_index, room.entered, position, yaw);
        }
        DragonMiniboss::FrozenAdventurer => {
            crate::server::entity::dungeon_mobs::spawner::spawn_frozen_adventurer(world, room_index, room.entered, position, yaw);
        }
        DragonMiniboss::AngryArchaeologist => {
            crate::server::entity::dungeon_mobs::spawner::spawn_angry_archaeologist(world, room_index, room.entered, position, yaw);
        }
    }
}

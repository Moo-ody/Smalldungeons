//! Shadow Assassin: the Champion-room miniboss for the room named "Shadow Assassin" - invisible,
//! permanently wearing dark purple leather boots (armor stays visible on an invisible entity,
//! same as vanilla), wielding an iron sword. Spawned once on room entry, same timing as Three
//! Weirdos/Water Board's gate roll (`Dungeon::tick`'s room-entry hook) rather than at room load,
//! so he doesn't exist yet for a player merely peeking in from an adjacent room.
//!
//! No real spawn was ever captured for this room (`room_data/mobs/shadow_assassin.json`'s
//! `spawns` array is empty - confirmed across every copy of the project's scraped data), so
//! there's no scraped position/equipment to replay the way every other room's mobs are spawned
//! (`spawner::spawn_room_mobs`). This is an ad-hoc spawn instead, same shape as
//! `spawner::spawn_crypt_undead`/`spawn_king_midas` - see `DungeonMobType::ShadowAssassin`'s own
//! doc comment. Position (15, 69, 15) and facing (north, room-relative) are given directly, not
//! captured. Fighting mechanics (ambush teleport, strafe, melee) are real - see
//! `ai/profile.rs`'s `TeleportAmbush` entry for this archetype and `ai/mod.rs`'s dedicated
//! branch for it; a real mob-vs-player damage system doesn't exist yet for any dungeon mob
//! though, so he can currently be fought but not actually killed.
//!
//! This same room-entry hook fires for a single-room `/practice` dungeon exactly like a real
//! one (`dungeon::practice` reuses `Dungeon::tick`'s whole room-entry pass unmodified - it isn't
//! gated behind `Dungeon::practice_room` the way ordinary room-JSON mob spawning is), so
//! `/practice shadow_assassin <door> <secrets>` spawns him too, with no separate wiring needed.

use crate::dungeon::room::room::Room;
use crate::server::block::block_position::BlockPos;
use crate::server::block::rotatable::Rotatable;
use crate::server::world::World;

/// Room-relative (pre-rotation) spawn position, given directly (not captured - see the module
/// doc comment).
const SPAWN_POSITION: BlockPos = BlockPos { x: 15, y: 69, z: 15 };

/// Sets up the Shadow Assassin miniboss for `room` if it actually is that room. No-op for every
/// other room (belt-and-suspenders check; the caller already filters by name and by
/// `Room::shadow_assassin_spawned` before calling this at all).
pub fn setup(room: &Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Shadow Assassin" {
        return;
    }

    let world_pos = room.get_world_block_pos(&SPAWN_POSITION);
    let position = crate::server::utils::dvec3::DVec3::new(world_pos.x as f64 + 0.5, world_pos.y as f64, world_pos.z as f64 + 0.5);
    let yaw = 180.0_f32.rotate(room.rotation);

    crate::server::entity::dungeon_mobs::spawner::spawn_shadow_assassin(world, room_index, room.entered, position, yaw);
}

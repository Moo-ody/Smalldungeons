//! Lets dungeon mobs actually be killed. There is no general mob HP/damage system anywhere
//! in this codebase (mob nametags show a *display* HP from `DungeonMobType::base_health`,
//! but nothing decrements it) - building a full damage model is out of scope here, so this
//! wires an explicit, simple rule instead: hitting a dungeon mob with either of the two
//! requested endgame weapons kills it outright.
//!
//! `kill_mob` (death animation + species-specific sound + despawn) is shared with
//! `hyperion::handle_hyperion_explosion`'s AOE kill path - both Hyperion and Spirit Sceptre
//! (whose bat projectile calls into that same function) route through here.

use crate::net::protocol::play::clientbound::{EntityStatus, SoundEffect};
use crate::server::entity::entity::{Entity, EntityId};
use crate::server::entity::entity_metadata::EntityVariant;
use crate::server::items::Item;
use crate::server::player::inventory::ItemSlot;
use crate::server::player::player::Player;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;

/// `EntityStatus` code for the generic "entity died" animation - the client plays the usual
/// death animation/particles for whatever entity type this is on its own.
const ENTITY_DEATH_STATUS: i8 = 3;

/// Whether `player`'s currently-held item should instantly kill any dungeon mob it hits.
fn wields_lethal_weapon(player: &Player) -> bool {
    matches!(
        player.inventory.get_hotbar_slot(player.held_slot as usize),
        Some(ItemSlot::Filled(Item::Hyperion | Item::SpiritSceptre, _))
    )
}

/// Kills `entity` outright if `player` hit it while holding a lethal weapon.
pub fn apply_lethal_hit(entity: &mut Entity, player: &Player) {
    if !wields_lethal_weapon(player) {
        return;
    }
    let entity_id = entity.id;
    kill_mob(entity.world_mut(), entity_id);
}

/// Plays the death animation + a species-appropriate death sound for `entity_id`, then
/// despawns it (which also cleans up equipment/nametag/AI state via `World::tick`'s removal
/// loop). Looks up the entity's variant itself so every caller gets the right sound "for
/// free" instead of each hardcoding one.
pub fn kill_mob(world: &mut World, entity_id: EntityId) {
    let Some((entity, _)) = world.entities.get(&entity_id) else { return };
    let pos = entity.position;
    let sound = death_sound_for(&entity.metadata.variant);

    for player in world.players.values_mut() {
        player.write_packet(&EntityStatus {
            entity_id,
            logic_op_code: ENTITY_DEATH_STATUS,
        });
        player.write_packet(&SoundEffect {
            sound: sound.id(),
            pos_x: pos.x,
            pos_y: pos.y,
            pos_z: pos.z,
            volume: 1.0,
            pitch: 1.0,
        });
    }

    world.despawn_entity(entity_id);

    // If this was a tracked starred mob, its room owes one less death before the map can show
    // a checkmark - redraw the map once the last one is gone so the change is visible right
    // away instead of waiting for some unrelated map update.
    if let Some(room_index) = world.entity_starred_mob_room.remove(&entity_id) {
        let dungeon = &mut world.server_mut().dungeon;
        if let Some(room) = dungeon.rooms.get_mut(room_index) {
            room.starred_mobs_remaining = room.starred_mobs_remaining.saturating_sub(1);
            if room.starred_mobs_remaining == 0 {
                dungeon.update_map_for_room(room_index);
            }
        }
    }
}

fn death_sound_for(variant: &EntityVariant) -> Sounds {
    match variant {
        EntityVariant::Skeleton { .. } => Sounds::SkeletonDeath,
        EntityVariant::Enderman { .. } => Sounds::EndermenDeath,
        // Player-model dungeon NPCs use the zombie death sound too, per explicit request,
        // rather than a "real" player hurt/death sound.
        _ => Sounds::ZombieDeath,
    }
}

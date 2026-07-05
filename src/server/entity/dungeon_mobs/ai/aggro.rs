//! Local aggro propagation: a mob acquires a target by seeing it (perception) or being hit
//! (`on_mob_attacked`), and other dormant/idle mobs *nearby* react too - not the whole room.

use crate::server::entity::dungeon_mobs::ai::profile::profile_for;
use crate::server::entity::dungeon_mobs::ai::state::ActivationState;
use crate::server::entity::entity::{Entity, EntityId};
use crate::server::player::player::ClientId;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;

/// Gives every mob within `radius` of `source_pos` that doesn't already have a target the
/// same `target`, marking them `Combat` with a pending first-sight nudge. Mobs already
/// fighting something else are left alone. O(n^2) over `world.entities` per propagation
/// event is fine at dungeon-room mob counts (dozens, not thousands).
pub fn acquire_or_propagate_target(world: &mut World, source_pos: DVec3, target: ClientId, radius: f64) {
    let nearby_ids: Vec<EntityId> = world.entities.iter()
        .filter(|(_, (entity, _))| entity.position.distance_to(&source_pos) <= radius)
        .map(|(id, _)| *id)
        .collect();

    for id in nearby_ids {
        if let Some(state) = world.entity_mob_ai.get_mut(&id) {
            if state.target.is_none() {
                state.target = Some(target);
                state.activation = ActivationState::Combat;
                state.first_sight_pending = true;
            }
        }
    }
}

/// Placeholder "this mob was hit" aggro trigger, wired to the existing
/// `EntityImpl::interact`/`EntityInteractionType::Attack` hook - there is no real HP/damage
/// system yet for mobs to react to actual damage, so a melee hit stands in for it.
pub fn on_mob_attacked(entity: &mut Entity, attacker: ClientId) {
    let entity_id = entity.id;
    let source_pos = entity.position;
    let world = entity.world_mut();

    let Some(state) = world.entity_mob_ai.get_mut(&entity_id) else { return };
    state.target = Some(attacker);
    state.activation = ActivationState::Combat;
    state.first_sight_pending = true;
    let radius = profile_for(state.archetype).aggro_propagation_radius;

    acquire_or_propagate_target(world, source_pos, attacker, radius);
}

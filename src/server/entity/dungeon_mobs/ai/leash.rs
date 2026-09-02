//! Spawn leash + return-to-spawn. Also the shared mechanism `green_room.rs` uses to force a
//! disengage - both "wandered too far" and "target reached a green room" boil down to the
//! same "give up and walk home" behavior, so they share one return-to-spawn code path.

use crate::server::entity::dungeon_mobs::ai::state::{ActivationState, MobAiState};
use crate::server::utils::dvec3::DVec3;

/// How close to `spawn_origin` counts as "home" and clears the return-to-spawn state.
const RETURN_ARRIVAL_DISTANCE: f64 = 2.0;

/// Checks the leash distance (if this archetype has one - `None` means ignore it entirely,
/// per Fels) and updates `state.leashed_out`/`state.target` accordingly. Call once per AI
/// think-tick whenever the mob has a target or is already returning home.
pub fn tick_leash(state: &mut MobAiState, current_pos: DVec3, leash_distance: Option<f64>) {
    if state.leashed_out {
        if current_pos.distance_to(&state.spawn_origin) <= RETURN_ARRIVAL_DISTANCE {
            state.leashed_out = false;
            state.activation = ActivationState::Idle;
        }
        return;
    }

    if let Some(max_distance) = leash_distance {
        if current_pos.distance_to(&state.spawn_origin) > max_distance {
            disengage(state);
        }
    }
}

/// Forces a mob to stop chasing/attacking and head back toward its spawn point. Used when the
/// leash distance is exceeded, the target disengages via a green room, or LOS to the target has
/// been lost long enough to give up (see `ai/mod.rs`'s `LOS_FORGET_TICKS`). Sets `activation`
/// back to `Idle` immediately (not just once arrived, like `tick_leash`'s own arrival branch
/// does) so the mob's active-mode visuals (raised arms, sprint - see `ai/mod.rs`) drop right
/// away too, matching "shouldn't fight the active mode" during the return trip.
pub fn disengage(state: &mut MobAiState) {
    state.target = None;
    state.leashed_out = true;
    state.activation = ActivationState::Idle;
}

//! Per-entity AI state, stored in a single `World.entity_mob_ai` map rather than one map per
//! "reusable component" - see `ai/mod.rs` module docs for why. Reusability of *behavior* is
//! achieved by the separate modules/functions that read and write these fields, not by
//! splitting the storage itself.

use crate::server::entity::dungeon_mobs::ai::cooldown::Cooldown;
use crate::server::entity::dungeon_mobs::mob_type::DungeonMobType;
use crate::server::player::player::ClientId;
use crate::server::utils::dvec3::DVec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationState {
    /// Beyond idle-activation range - fully asleep, no wandering, no perception checks.
    Dormant,
    /// Within idle-activation range but no combat target - may look around/wander near spawn.
    Idle,
    /// Has a target and is pursuing/attacking it.
    Combat,
}

/// Named cooldown slots for a mob's abilities. Two is enough for every archetype in the
/// user's notes (e.g. Skeleton Master's normal arrow + independent wither-skull timer) -
/// `secondary` simply stays unused (`Cooldown::ready()`) until a Phase 3+ archetype needs it.
#[derive(Debug, Clone, Copy, Default)]
pub struct AttackCooldowns {
    pub primary: Cooldown,
    pub secondary: Cooldown,
}

/// Every field is `Copy` on purpose: the AI pipeline reads a snapshot out of
/// `World.entity_mob_ai`, mutates the local copy while freely calling functions that need
/// `&mut World` (perception, aggro propagation, projectile spawning, ...), then writes the
/// result back at the end - sidestepping the multi-field-borrow conflict that would come
/// from holding a live `&mut MobAiState` borrow into `World` across those calls.
#[derive(Clone, Copy)]
pub struct MobAiState {
    pub archetype: DungeonMobType,
    pub spawn_origin: DVec3,
    pub spawn_yaw: f32,

    pub activation: ActivationState,

    /// Current combat target, if any. Dungeon mobs only ever target players, never each
    /// other, so this is keyed by `ClientId` (matching how every other lookup - nearest
    /// player, LOS, distance - is already keyed elsewhere in this codebase), not the
    /// existing-but-unused `EntityId`-keyed `entity_current_target` component.
    pub target: Option<ClientId>,
    /// Set for one think-cycle when a target is first acquired - lets movement give the
    /// documented "first-sight forward nudge" before settling into normal behavior.
    pub first_sight_pending: bool,

    /// True while returning to `spawn_origin` after exceeding the leash distance or losing
    /// the target to a green-room/vision disengagement. Cleared once back near spawn.
    pub leashed_out: bool,

    pub idle_wander_target: Option<DVec3>,
    pub idle_wander_cooldown: Cooldown,

    pub attack_cooldowns: AttackCooldowns,
}

impl MobAiState {
    pub fn new(archetype: DungeonMobType, spawn_origin: DVec3, spawn_yaw: f32) -> Self {
        Self {
            archetype,
            spawn_origin,
            spawn_yaw,
            activation: ActivationState::Dormant,
            target: None,
            first_sight_pending: false,
            leashed_out: false,
            idle_wander_target: None,
            idle_wander_cooldown: Cooldown::ready(),
            attack_cooldowns: AttackCooldowns::default(),
        }
    }
}

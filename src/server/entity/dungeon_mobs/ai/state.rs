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
    /// The room this mob was spawned into - purely a lookup key `Dungeon::tick` uses to find
    /// every mob belonging to a room the instant that room is actually entered (see
    /// `room_entered`'s own doc comment for what that then does); `ai/mod.rs::run_mob_ai` itself
    /// never reads this field or looks the room up.
    pub room_index: usize,
    /// Whether this mob's own room (`room_index`) has actually been entered by a player yet -
    /// pushed directly to `true`, once, by `Dungeon::tick` the exact tick that happens (iterating
    /// every mob whose `room_index` matches), rather than `ai/mod.rs::run_mob_ai` re-deriving it
    /// every tick via `world.server_mut().dungeon.rooms.get(room_index)`. That would need a real
    /// `Server`/`Dungeon` reachable from every single mob tick, including a fully idle one -
    /// exactly what `perf_bench.rs`'s own doc comment says must never be required (its benchmark
    /// intentionally leaves `world.server` null for idle mobs). Starts `false` (matches spawning
    /// dormant-until-entered, see `MobAiState::new`'s own default `ActivationState::Dormant`);
    /// `run_mob_ai` forces `ActivationState::Dormant` (skipping perception/idle-wander entirely)
    /// regardless of distance while this is still `false` - see the module's own doc comment for
    /// the full dormant-pre-spawn feature this backs.
    pub room_entered: bool,

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

    /// Ticks remaining in a ranged attacker's current bow-draw/skull-nock wind-up - 0 means not
    /// currently drawing. See `attack::try_ranged`; only meaningful for `AttackModule::Ranged`/
    /// `HybridMeleeRanged` archetypes.
    pub bow_draw_ticks: u32,

    /// Last arm-raised pose actually sent to players (`Zombie`'s `is_attacking` metadata bit,
    /// repurposed by `ai/mod.rs` as a persistent "detected the player" pose rather than a brief
    /// per-swing pulse) - tracked so metadata is only resent on an actual change, not every tick.
    pub arms_raised: bool,
    /// Last sprint pose actually sent to players (`EntityMetadata::is_sprinting`) - same
    /// change-tracking purpose as `arms_raised`, for humanoid NPC-model archetypes' combat
    /// sprint (see `ai/mod.rs`).
    pub sprinting: bool,

    /// Throttles the (block-sampled raycast, not free) line-of-sight re-check on an existing
    /// combat target to once every `LOS_CHECK_INTERVAL_TICKS` instead of every tick.
    pub los_check_cooldown: Cooldown,
    /// Ticks since LOS to the current target was last actually confirmed - reset to 0 whenever
    /// a check (throttled by `los_check_cooldown`) finds it, incremented by the check interval
    /// each time one doesn't. Once this reaches `LOS_FORGET_TICKS`, the mob gives up ("relatively
    /// oblivious") and returns to spawn - see `ai/mod.rs`.
    pub ticks_since_los: u32,

    /// Only meaningful for an archetype with `AiProfile::teleport_ambush` set (currently just
    /// Shadow Assassin) - ticks left frozen (no movement, still invisible) after first
    /// acquiring a target, before the initial ambush teleport. Seeded from
    /// `TeleportAmbush::stun_ticks` the tick a target is acquired; 0 once the freeze has ended
    /// (or for any archetype with no `teleport_ambush` profile, where it's simply never set).
    pub stun_ticks: u32,
    /// Only meaningful alongside `stun_ticks` above - counts down to the next scheduled
    /// teleport-behind while aggroed (`TeleportAmbush::reengage_interval_ticks`).
    pub teleport_cooldown: Cooldown,

    /// Only meaningful for a `teleport_ambush` archetype (Shadow Assassin) - `true` orbits its
    /// target clockwise, `false` counter-clockwise. Rolled once per engagement (the ambush
    /// teleport that first sets `is_invisible = false`) and held fixed for the rest of that
    /// fight - see `ai/mod.rs`'s `teleport_ambush` branch.
    pub orbit_clockwise: bool,
    /// Ticks until the next volley-chance roll while orbiting normally (it attacks continuously
    /// every tick otherwise - see `ai/mod.rs`'s `teleport_ambush` branch). Only meaningful for a
    /// `teleport_ambush` archetype.
    pub weave_cooldown: Cooldown,
    /// > 0 while mid bow-volley (arrows still left to fire this burst) - movement is fully
    /// frozen while this is nonzero.
    pub volley_arrows_left: u8,
    /// Ticks until the volley's next arrow.
    pub volley_shot_cooldown: Cooldown,

    /// Only meaningful for a `room_confined` archetype (see `AiProfile::room_confined`'s own
    /// doc comment) - `true` once a player has actually landed a hit on it
    /// (`combat::on_player_damaged_mob`), permanently lifting its room confinement. Never reset
    /// back to `false` - even after an auto-return teleport (see `no_interaction_ticks` below),
    /// it stays released.
    pub damaged_once: bool,
    /// Only meaningful once `damaged_once` - ticks since any player was last in this mob's line
    /// of sight *or* damage was dealt in either direction. Reset to 0 by either; once it reaches
    /// the auto-return threshold (`ai/mod.rs`'s `NO_INTERACTION_RETURN_TICKS`), the mob
    /// teleports back to `spawn_origin` and this resets to 0 again.
    pub no_interaction_ticks: u32,
}

impl MobAiState {
    pub fn new(archetype: DungeonMobType, spawn_origin: DVec3, spawn_yaw: f32, room_index: usize, room_entered: bool) -> Self {
        Self {
            archetype,
            spawn_origin,
            spawn_yaw,
            room_index,
            room_entered,
            activation: ActivationState::Dormant,
            target: None,
            first_sight_pending: false,
            leashed_out: false,
            idle_wander_target: None,
            idle_wander_cooldown: Cooldown::ready(),
            attack_cooldowns: AttackCooldowns::default(),
            bow_draw_ticks: 0,
            arms_raised: false,
            sprinting: false,
            los_check_cooldown: Cooldown::ready(),
            ticks_since_los: 0,
            stun_ticks: 0,
            teleport_cooldown: Cooldown::ready(),
            orbit_clockwise: false,
            weave_cooldown: Cooldown::ready(),
            volley_arrows_left: 0,
            volley_shot_cooldown: Cooldown::ready(),
            damaged_once: false,
            no_interaction_ticks: 0,
        }
    }
}

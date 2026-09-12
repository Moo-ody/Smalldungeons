//! `AiProfile` is where a [`DungeonMobType`] archetype is *composed* from the reusable
//! behavior pieces (vision, leash, movement style, attack module) - the one place per-mob
//! differences live, as a single small match statement rather than per-mob branching spread
//! through the AI pipeline itself.
//!
//! This intentionally lives here rather than on `DungeonMobType` in `mob_type.rs`: that file
//! is pure cosmetic/model data (skin, HP display, base entity kind) with no knowledge of AI
//! concepts, and should stay that way so nothing there ever needs to depend on `ai::*` types.

use crate::server::entity::dungeon_mobs::ai::attack::{AttackModule, RangedProjectile, MELEE_COOLDOWN_TICKS};
use crate::server::entity::dungeon_mobs::ai::movement::MovementStyle;
use crate::server::entity::dungeon_mobs::mob_type::DungeonMobType;

/// Default idle-activation range (`~46-48 blocks` per the documented behavior) - distinct
/// from combat vision range, which is much shorter and per-archetype.
pub const IDLE_ACTIVATION_RANGE: f64 = 47.0;

/// Default local aggro-propagation radius: how close another dormant/idle mob needs to be to
/// react when a nearby mob notices or is attacked. Tunable per archetype later if needed.
pub const DEFAULT_AGGRO_PROPAGATION_RADIUS: f64 = 8.0;

/// Default spawn leash distance. `None` on a profile means "ignore the leash entirely"
/// (documented for Fels - a Phase 5 archetype, not implemented yet, but the field already
/// supports it).
pub const DEFAULT_LEASH_DISTANCE: f64 = 20.0;

/// Shadow Assassin's documented ambush pattern: freeze invisible after first spotting a target,
/// then teleport directly behind them (becoming visible) instead of walking up - re-triggered
/// periodically (or immediately if the target gets far enough away) while still aggroed, laid
/// on top of the normal `movement`/`attack` modules below (which drive the ordinary
/// strafe-and-swing combat once engaged). `None` on every other archetype's profile.
#[derive(Clone, Copy)]
pub struct TeleportAmbush {
    /// Ticks frozen (no movement, still invisible) after first acquiring a target, before the
    /// initial ambush teleport.
    pub stun_ticks: u32,
    /// How often (ticks), while still aggroed, it re-teleports directly behind its target even
    /// if already close/fighting.
    pub reengage_interval_ticks: u32,
    /// If the target gets at least this far away while aggroed, teleport behind them
    /// immediately instead of waiting for `reengage_interval_ticks`.
    pub reengage_distance: f64,
}

#[derive(Clone, Copy)]
pub struct AiProfile {
    /// Whether this archetype has real AI behavior yet. `false` archetypes are left exactly
    /// as they behave today (inert, `ai_disabled = true`) rather than guessing at
    /// unspecified/complex Hypixel mechanics.
    pub implemented: bool,
    pub vision_range: f64,
    pub requires_los: bool,
    pub aggro_propagation_radius: f64,
    pub leash_distance: Option<f64>,
    pub movement: MovementStyle,
    pub attack: AttackModule,
    pub speed_multiplier: f64,
    /// See `TeleportAmbush`'s own doc comment. `None` for every archetype except Shadow
    /// Assassin today.
    pub teleport_ambush: Option<TeleportAmbush>,
    /// Periodically forces a real jump (see `physics::try_jump`) while genuinely chasing a
    /// target from far away (not while holding a ranged stance, and not once already close) -
    /// "run and jump to the player until they get close" per explicit request, giving a
    /// player-model archetype a bounding/parkour-ish charge instead of a flat glide.
    pub hops_while_approaching: bool,
    /// Yellow-room ("Champion room") miniboss containment, per explicit request: `true`
    /// archetypes refuse to engage a target standing outside their own spawn room
    /// (`MobAiState::room_index`) at all - no movement, no attack - until a player actually
    /// lands a hit on them (`MobAiState::damaged_once`, set by
    /// `combat::on_player_damaged_mob`), which permanently lifts the room confinement. Once
    /// released, `ai/mod.rs`'s own containment check separately auto-teleports it back to
    /// `MobAiState::spawn_origin` after 100 ticks with no LOS to any player and no damage dealt
    /// in either direction (`MobAiState::no_interaction_ticks`) - see `run_mob_ai`'s own
    /// containment block for both halves of this mechanic.
    pub room_confined: bool,
}

impl AiProfile {
    /// Baseline used as the `..` fallback when building a real profile below.
    fn defaults() -> Self {
        Self {
            implemented: true,
            vision_range: 32.0,
            requires_los: true,
            aggro_propagation_radius: DEFAULT_AGGRO_PROPAGATION_RADIUS,
            leash_distance: Some(DEFAULT_LEASH_DISTANCE),
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5, cooldown_ticks: MELEE_COOLDOWN_TICKS },
            speed_multiplier: 1.0,
            teleport_ambush: None,
            hops_while_approaching: false,
            room_confined: false,
        }
    }

    /// Safe inert default for archetypes whose real behavior isn't implemented yet (Phase 3+
    /// in the design notes) - keeps today's "spawns but never moves" behavior rather than
    /// inventing unverified Hypixel mechanics.
    fn placeholder() -> Self {
        Self { implemented: false, ..Self::defaults() }
    }
}

/// Composes an [`AiProfile`] for an archetype. This is the single place new mob behavior
/// gets wired in - add a match arm here, everything else in `ai/` is already reusable.
pub fn profile_for(archetype: DungeonMobType) -> AiProfile {
    use DungeonMobType::*;

    match archetype {
        // BASIC_ZOMBIE: detect, approach, basic melee. Zombie Lord shares the exact same
        // controller with only a speed multiplier - not a separate archetype/AI.
        SuperTankZombie | ZombieSoldier | ZombieKnight | ZombieLord | CryptLurker => AiProfile {
            vision_range: 32.0,
            requires_los: true,
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5, cooldown_ticks: MELEE_COOLDOWN_TICKS },
            speed_multiplier: if archetype == ZombieLord { 1.2 } else { 1.0 },
            ..AiProfile::defaults()
        },

        // SKELETON_RANGED: try to hold ~15 blocks, approach if too far, fire an arrow every
        // 30 ticks. Skeleton Master/Super Archer's extra abilities (wither skull, bouncing
        // arrows) are Phase 3+ additions to this same base - not implemented yet.
        SkeletonSoldier | SkeletonMaster | SkeletonLord | SuperArcher => AiProfile {
            vision_range: 32.0,
            requires_los: true,
            movement: MovementStyle::MaintainDistance { preferred: 15.0, tolerance: 2.0 },
            attack: AttackModule::Ranged { speed_bps: 20.0, cooldown_ticks: 30, range: 17.0 },
            ..AiProfile::defaults()
        },

        // Sniper: extreme vision range, otherwise still a placeholder until its stationary
        // return-to-origin controller (Phase 4) is built.
        Sniper => AiProfile { vision_range: 128.0, ..AiProfile::placeholder() },

        // Crypt Dreadlord: purely melee (iron sword) - approaches like a basic zombie, but once
        // within striking range circle-strafes around the player instead of just standing/
        // pushing in, per the documented "strafes while keeping aim locked on" behavior. Still
        // on the player-model NPC path (see `spawn_as_npc`).
        CryptDreadlord => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::CircleStrafe { melee_range: 2.5, orbit_radius: 2.5 },
            attack: AttackModule::Melee { range: 2.5, cooldown_ticks: MELEE_COOLDOWN_TICKS },
            ..AiProfile::defaults()
        },

        // Crypt Souleater: hybrid - wither-skull ranged beyond 5 blocks, diamond-sword melee
        // within it. Movement and attack share the same `melee_range` so they switch state
        // together.
        CryptSouleater => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::HybridMeleeRanged { melee_range: 5.0, ranged_preferred: 10.0, ranged_tolerance: 6.0 },
            attack: AttackModule::HybridMeleeRanged { melee_range: 5.0, ranged_speed_bps: 20.0, ranged_cooldown_ticks: 20, ranged_range: 16.0, projectile: RangedProjectile::WitherSkull },
            ..AiProfile::defaults()
        },

        // Crypt Undead: same melee shape as Crypt Dreadlord (per user request - "crypt
        // dreadlord mechanics"). Appears both in ordinary room-JSON spawns and ad hoc at an
        // exploded Crypt's position (`spawner::spawn_crypt_undead`, holding a Bone instead of
        // a sword - see `spawner::crypt_undead_equipment`) once a player detonates one.
        CryptUndead => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5, cooldown_ticks: MELEE_COOLDOWN_TICKS },
            ..AiProfile::defaults()
        },

        // Withermancer: shoulder-skull charges (remaining_skulls = 2) aren't implemented yet
        // (Phase 3), but it shouldn't just stand there doing nothing in the meantime - falls
        // back to its documented post-skulls behavior (stone sword melee) as a simple
        // placeholder until the skull-charge ability itself is built.
        Withermancer => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5, cooldown_ticks: MELEE_COOLDOWN_TICKS },
            speed_multiplier: 1.5,
            ..AiProfile::defaults()
        },

        // Fels ignores the leash entirely (per the notes) and is 3x normal mob speed. Its
        // dedicated sprint-jump/weave/LOS-disengage movement controller is Phase 5 and not
        // implemented yet - basic approach + melee is a placeholder so it at least fights
        // back in the meantime, same reasoning as Withermancer above.
        Fels => AiProfile {
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5, cooldown_ticks: MELEE_COOLDOWN_TICKS },
            leash_distance: None,
            speed_multiplier: 3.0,
            ..AiProfile::defaults()
        },

        // Zombie Commander: casts a fishing hook and reels the player in on contact rather than
        // firing arrows, per the documented "fishing rod cast every 10 ticks, hold ~6 blocks"
        // behavior - see `AttackModule::FishingRod`/`ai::projectile::fire_fishing_hook_at_player`.
        ZombieCommander => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::MaintainDistance { preferred: 6.0, tolerance: 1.0 },
            attack: AttackModule::FishingRod { cast_speed_bps: 20.0, cooldown_ticks: 20, range: 7.0 },
            ..AiProfile::defaults()
        },

        // Lost Adventurer / Angry Archaeologist / Frozen Adventurer: all documented as bow
        // users with high mobility ("Lost Adventurer which wears Frozen Blaze Armor" etc) - per
        // explicit request, bow from range but switch to melee (sword) once a player closes in,
        // same hybrid shape Crypt Souleater already uses just with a real arrow instead of a
        // wither skull (see `RangedProjectile`/`try_ranged`'s `uses_bow` - this one actually
        // raises/lowers a bow for the shot). Their real special abilities (healing, freeze,
        // dragon-armor perks) are Phase 4+ and not implemented yet.
        // `movement` closes in (not `HybridMeleeRanged`'s hold-at-range pattern - fixed after an
        // explicit follow-up report: with a `ranged_preferred`/`ranged_tolerance` hold zone, he'd
        // settle in and camp around ~9-10 blocks out, shooting forever, instead of actually
        // running the rest of the way to melee), then once within `melee_range` a slow left-right
        // wiggle (`LightStrafe`, not `CircleStrafe`'s full orbit - per explicit follow-up
        // correction: "shouldn't try to strafe around me totally... strafing left and right a
        // bit") toward a tight ~1-block `orbit_radius`, backing off if it ends up standing right
        // on the player. This is also what stops `hops_while_approaching` below jumping into the
        // player once close - see its `still_closing` check. `attack` (still `HybridMeleeRanged`)
        // already fires arrows while still outside `melee_range` and switches to melee once
        // inside it, on top of this; `ai/mod.rs`'s `bow_draw_ticks` freeze (separate from this
        // `movement` field entirely) is what stops it still advancing while actually mid-shot.
        LostAdventurer | AngryArchaeologist | FrozenAdventurer => AiProfile {
            vision_range: 24.0,
            movement: MovementStyle::LightStrafe { melee_range: 2.5, orbit_radius: 1.0 },
            attack: AttackModule::HybridMeleeRanged { melee_range: 2.5, ranged_speed_bps: 24.0, ranged_cooldown_ticks: 25, ranged_range: 12.0, projectile: RangedProjectile::Arrow },
            hops_while_approaching: true,
            room_confined: true,
            ..AiProfile::defaults()
        },

        // Mimic: same basic detect/approach/melee controller as the plain zombies - it's a
        // disguised baby zombie underneath, not a distinct combat pattern. Real Hypixel attack
        // range isn't verified, so this reuses the standard melee shape rather than guessing
        // bespoke numbers - speed_multiplier is the one deliberately-set value (2.5x a normal
        // zombie, per request).
        Mimic => AiProfile {
            vision_range: 32.0,
            requires_los: true,
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5, cooldown_ticks: MELEE_COOLDOWN_TICKS },
            speed_multiplier: 2.5,
            ..AiProfile::defaults()
        },

        // King Midas: same melee shape as Crypt Dreadlord/Crypt Undead - approaches and
        // swings his golden sword. His actual "mechanic" (armor breaking off per hit, dying on
        // the 5th) lives in `ai/combat.rs::apply_king_midas_hit`, not here.
        KingMidas => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5, cooldown_ticks: MELEE_COOLDOWN_TICKS },
            ..AiProfile::defaults()
        },

        // Shadow Assassin: same detection range/LOS requirement as this codebase's other
        // player-model NPCs (Crypt Dreadlord/Undead, King Midas - vision_range 16.0), but once
        // it acquires a target it doesn't approach on foot at all - see `TeleportAmbush`'s doc
        // comment and `ai/mod.rs`'s dedicated branch for it. `movement`/`attack` below are what
        // drives it once actually engaged (strafe + melee), same shape as Crypt Dreadlord.
        // Backstab bonus / no-damage-reduction aren't implemented yet (no real mob-vs-player
        // damage system exists at all yet - see `attack::try_melee`'s own doc comment).
        ShadowAssassin => AiProfile {
            vision_range: 16.0,
            requires_los: true,
            // `movement` is never actually read for a `teleport_ambush` archetype - `ai/mod.rs`'s
            // dedicated branch drives its orbit/weave/volley movement directly
            // (`movement::orbit_and_weave`) instead of the generic `apply_movement_style` path.
            // Left as a plain `CircleStrafe` here only so the field has *some* value - it's
            // otherwise unused for this archetype.
            movement: MovementStyle::CircleStrafe { melee_range: 2.5, orbit_radius: 2.5 },
            // Attacks continuously while orbiting now (see `ai/mod.rs`'s `teleport_ambush`
            // branch), so this cooldown alone paces the actual hit rate - lowered from an
            // earlier 10 per explicit follow-up correction ("way more offensive... hitting me
            // almost constantly").
            attack: AttackModule::Melee { range: 2.5, cooldown_ticks: 6 },
            speed_multiplier: 1.5,
            teleport_ambush: Some(TeleportAmbush {
                stun_ticks: 40,
                reengage_interval_ticks: 120,
                reengage_distance: 20.0,
            }),
            room_confined: true,
            ..AiProfile::defaults()
        },
    }
}

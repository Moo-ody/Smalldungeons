//! `AiProfile` is where a [`DungeonMobType`] archetype is *composed* from the reusable
//! behavior pieces (vision, leash, movement style, attack module) - the one place per-mob
//! differences live, as a single small match statement rather than per-mob branching spread
//! through the AI pipeline itself.
//!
//! This intentionally lives here rather than on `DungeonMobType` in `mob_type.rs`: that file
//! is pure cosmetic/model data (skin, HP display, base entity kind) with no knowledge of AI
//! concepts, and should stay that way so nothing there ever needs to depend on `ai::*` types.

use crate::server::entity::dungeon_mobs::ai::attack::AttackModule;
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
            attack: AttackModule::Melee { range: 2.5 },
            speed_multiplier: 1.0,
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
            attack: AttackModule::Melee { range: 2.5 },
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
            attack: AttackModule::Ranged { speed_bps: 20.0, cooldown_ticks: 30 },
            ..AiProfile::defaults()
        },

        // Sniper: extreme vision range, otherwise still a placeholder until its stationary
        // return-to-origin controller (Phase 4) is built.
        Sniper => AiProfile { vision_range: 128.0, ..AiProfile::placeholder() },

        // Crypt Dreadlord: purely melee (iron sword) - approaches and swings like a basic
        // zombie, just on the player-model NPC path (see `spawn_as_npc`).
        CryptDreadlord => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5 },
            ..AiProfile::defaults()
        },

        // Crypt Souleater: hybrid - wither-skull ranged beyond 5 blocks, diamond-sword melee
        // within it. Movement and attack share the same `melee_range` so they switch state
        // together.
        CryptSouleater => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::HybridMeleeRanged { melee_range: 5.0, ranged_preferred: 10.0, ranged_tolerance: 6.0 },
            attack: AttackModule::HybridMeleeRanged { melee_range: 5.0, ranged_speed_bps: 20.0, ranged_cooldown_ticks: 20 },
            ..AiProfile::defaults()
        },

        // Crypt Undead: same melee shape as Crypt Dreadlord (per user request - "crypt
        // dreadlord mechanics"). Appears both in ordinary room-JSON spawns and ad hoc at an
        // exploded Crypt's position (`spawner::spawn_crypt_undead`, holding a Bone instead of
        // a sword - see `spawner::crypt_undead_equipment`) once a player detonates one.
        CryptUndead => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5 },
            ..AiProfile::defaults()
        },

        // Withermancer: shoulder-skull charges (remaining_skulls = 2) aren't implemented yet
        // (Phase 3), but it shouldn't just stand there doing nothing in the meantime - falls
        // back to its documented post-skulls behavior (stone sword melee) as a simple
        // placeholder until the skull-charge ability itself is built.
        Withermancer => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5 },
            ..AiProfile::defaults()
        },

        // Fels ignores the leash entirely (per the notes) and is 3x normal mob speed. Its
        // dedicated sprint-jump/weave/LOS-disengage movement controller is Phase 5 and not
        // implemented yet - basic approach + melee is a placeholder so it at least fights
        // back in the meantime, same reasoning as Withermancer above.
        Fels => AiProfile {
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5 },
            leash_distance: None,
            speed_multiplier: 3.0,
            ..AiProfile::defaults()
        },

        // Zombie Commander: its real "fishing rod cast every 10 ticks, hold ~6 blocks"
        // distance-control/strafe controller is Phase 4 and not implemented yet - approximate
        // with the same maintain-distance ranged shape as the skeletons for now rather than
        // leaving it completely inert.
        ZombieCommander => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::MaintainDistance { preferred: 6.0, tolerance: 1.0 },
            attack: AttackModule::Ranged { speed_bps: 20.0, cooldown_ticks: 20 },
            ..AiProfile::defaults()
        },

        // Lost Adventurer / Angry Archaeologist / Frozen Adventurer: all documented as bow
        // users with high mobility ("Lost Adventurer which wears Frozen Blaze Armor" etc) -
        // their real special abilities (healing, freeze, dragon-armor perks) are Phase 4+ and
        // not implemented, but a basic ranged stance is a much closer placeholder than
        // standing completely still.
        LostAdventurer | AngryArchaeologist | FrozenAdventurer => AiProfile {
            vision_range: 24.0,
            movement: MovementStyle::MaintainDistance { preferred: 10.0, tolerance: 2.0 },
            attack: AttackModule::Ranged { speed_bps: 24.0, cooldown_ticks: 25 },
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
            attack: AttackModule::Melee { range: 2.5 },
            speed_multiplier: 2.5,
            ..AiProfile::defaults()
        },

        // King Midas: same melee shape as Crypt Dreadlord/Crypt Undead - approaches and
        // swings his golden sword. His actual "mechanic" (armor breaking off per hit, dying on
        // the 5th) lives in `ai/combat.rs::apply_king_midas_hit`, not here.
        KingMidas => AiProfile {
            vision_range: 16.0,
            movement: MovementStyle::Approach,
            attack: AttackModule::Melee { range: 2.5 },
            ..AiProfile::defaults()
        },
    }
}

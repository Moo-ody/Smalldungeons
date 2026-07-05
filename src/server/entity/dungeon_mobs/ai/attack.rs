//! Attack modules. Melee reuses the *existing* swing/cooldown machinery
//! (`CombatState`/`AttackCooldown`, already driven every tick by
//! `World::process_combat_state_system`) instead of a parallel system - that machinery is
//! currently only populated by the `/spawn` debug command's `spawn_equipped_zombie`, never
//! by dungeon mobs, so `spawner.rs` now also registers it for melee archetypes. Ranged uses
//! the new `Cooldown` primitive on `MobAiState` since there's no existing analog for it.

use crate::net::protocol::play::clientbound::SoundEffect;
use crate::server::entity::dungeon_mobs::ai::projectile::{fire_arrow_at_player, fire_wither_skull_at_player};
use crate::server::entity::dungeon_mobs::ai::state::MobAiState;
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_metadata::EntityVariant;
use crate::server::entity::spawn_equipped::AttackCooldown;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;
use rand::Rng;

#[derive(Clone, Copy)]
pub enum AttackModule {
    Melee { range: f64 },
    Ranged { speed_bps: f64, cooldown_ticks: u32 },
    /// Crypt Souleater's documented hybrid: ranged (wither skull) beyond `melee_range`,
    /// melee (diamond sword swing) within it. Movement needs to match - see
    /// `MovementStyle::HybridMeleeRanged`, driven by the same `melee_range`.
    HybridMeleeRanged { melee_range: f64, ranged_speed_bps: f64, ranged_cooldown_ticks: u32 },
}

/// Ticks the swing lasts once triggered - purely a visual arm-pose duration.
const MELEE_SWING_TICKS: u16 = 6;
/// Ticks between melee swings (~1 second at 20 TPS). Not specified precisely in the design
/// notes for BASIC_ZOMBIE; a reasonable placeholder pending real tuning.
const MELEE_COOLDOWN_TICKS: u16 = 20;
/// Approximate hand/eye height offsets so projectiles visibly leave from around a mob's
/// hands and aim at the target's torso/head, instead of spawning at/aiming at their feet
/// (which is what `entity.position`/`player.position` are).
const SHOOTER_HAND_HEIGHT: f64 = 1.4;
const TARGET_AIM_HEIGHT: f64 = 1.5;
/// Small random spread applied to a ranged cooldown reset so multiple mobs that all
/// acquired their target on the same tick (e.g. via aggro propagation) don't stay firing in
/// perfect lockstep forever after their first, simultaneous shot.
const RANGED_COOLDOWN_JITTER_TICKS: i32 = 4;

pub fn try_attack(world: &mut World, entity: &mut Entity, state: &mut MobAiState, module: AttackModule, target_pos: DVec3) {
    match module {
        AttackModule::Melee { range } => try_melee(world, entity, range, target_pos),
        AttackModule::Ranged { speed_bps, cooldown_ticks } => {
            try_ranged(world, entity, state, speed_bps, cooldown_ticks, target_pos, fire_arrow_at_player);
        }
        AttackModule::HybridMeleeRanged { melee_range, ranged_speed_bps, ranged_cooldown_ticks } => {
            if entity.position.distance_to(&target_pos) < melee_range {
                try_melee(world, entity, melee_range, target_pos);
            } else {
                try_ranged(world, entity, state, ranged_speed_bps, ranged_cooldown_ticks, target_pos, fire_wither_skull_at_player);
            }
        }
    }
}

fn try_melee(world: &mut World, entity: &mut Entity, range: f64, target_pos: DVec3) {
    if entity.position.distance_to(&target_pos) > range {
        return;
    }

    let entity_id = entity.id;
    let Some(cooldown) = world.get_attack_cooldown(entity_id) else { return };
    if cooldown.ticks > 0 {
        return;
    }

    world.set_attack_cooldown(entity_id, AttackCooldown { ticks: MELEE_COOLDOWN_TICKS });
    if let Some(combat_state) = world.get_combat_state_mut(entity_id) {
        combat_state.aggressive = true;
        combat_state.swing_ticks = MELEE_SWING_TICKS;
    }

    // No real mob-vs-player damage system exists yet (TODO) - the swing/arm-pose is the
    // visible "attack happened" placeholder, matching how `spawn_equipped_zombie` already
    // presents melee mobs.
    if let EntityVariant::Zombie { is_child, is_villager, is_converting, .. } = entity.metadata.variant {
        entity.metadata.variant = EntityVariant::Zombie {
            is_child,
            is_villager,
            is_converting,
            is_attacking: true,
        };
    }
    world.send_metadata_update(entity_id);
}

fn try_ranged(
    world: &mut World,
    entity: &mut Entity,
    state: &mut MobAiState,
    speed_bps: f64,
    cooldown_ticks: u32,
    target_pos: DVec3,
    fire: impl FnOnce(&mut World, DVec3, DVec3, f64) -> anyhow::Result<crate::server::entity::entity::EntityId>,
) {
    if !state.attack_cooldowns.primary.is_ready() {
        return;
    }

    let origin = DVec3::new(entity.position.x, entity.position.y + SHOOTER_HAND_HEIGHT, entity.position.z);
    let aim_point = DVec3::new(target_pos.x, target_pos.y + TARGET_AIM_HEIGHT, target_pos.z);

    if fire(world, origin, aim_point, speed_bps).is_ok() {
        let jitter = rand::rng().random_range(-RANGED_COOLDOWN_JITTER_TICKS..=RANGED_COOLDOWN_JITTER_TICKS);
        let jittered_cooldown = (cooldown_ticks as i32 + jitter).max(1) as u32;
        state.attack_cooldowns.primary.trigger(jittered_cooldown);

        for player in world.players.values_mut() {
            player.write_packet(&SoundEffect {
                sound: Sounds::Bow.id(),
                pos_x: origin.x,
                pos_y: origin.y,
                pos_z: origin.z,
                volume: 1.0,
                pitch: 1.0,
            });
        }
    }
}

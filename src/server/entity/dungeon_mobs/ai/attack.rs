//! Attack modules. Melee reuses the *existing* swing/cooldown machinery
//! (`CombatState`/`AttackCooldown`, already driven every tick by
//! `World::process_combat_state_system`) instead of a parallel system - that machinery is
//! currently only populated by the `/spawn` debug command's `spawn_equipped_zombie`, never
//! by dungeon mobs, so `spawner.rs` now also registers it for melee archetypes. Ranged uses
//! the new `Cooldown` primitive on `MobAiState` since there's no existing analog for it.

use crate::net::protocol::play::clientbound::SoundEffect;
use crate::server::entity::dungeon_mobs::ai::projectile::{fire_arrow_at_player, fire_fishing_hook_at_player, fire_wither_skull_at_player};
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
    /// `cooldown_ticks` is ticks between swings once in range - most archetypes just want
    /// `MELEE_COOLDOWN_TICKS`, but a faster/slower attacker (Shadow Assassin's documented
    /// "every 5 ticks", say) can override it per profile.
    Melee { range: f64, cooldown_ticks: u16 },
    /// `range` is the max distance a shot can actually be *taken* from - beyond it, any
    /// in-progress bow-draw/skull-nock is cancelled ("lowers the bow") instead of firing.
    /// Independent of `MovementStyle::MaintainDistance`'s `preferred`/`tolerance` (a different
    /// module, deliberately not coupled to this one) but set to roughly `preferred + tolerance`
    /// per archetype in `profile.rs` so the two line up in practice.
    Ranged { speed_bps: f64, cooldown_ticks: u32, range: f64 },
    /// Melee within `melee_range`, ranged (`projectile`) beyond it - Crypt Souleater's
    /// documented wither-skull hybrid, and now also Lost Adventurer/Angry Archaeologist/Frozen
    /// Adventurer's "bow far, melee close" behavior (same shape, different projectile/visual -
    /// see `RangedProjectile`). Movement needs to match - see `MovementStyle::HybridMeleeRanged`,
    /// driven by the same `melee_range`. `ranged_range` is the ranged half's own max engagement
    /// distance, same idea as `Ranged::range` above.
    HybridMeleeRanged { melee_range: f64, ranged_speed_bps: f64, ranged_cooldown_ticks: u32, ranged_range: f64, projectile: RangedProjectile },
    /// Zombie Commander's documented ability: casts a fishing hook rather than firing an arrow -
    /// "once it makes contact it reels it in" instead of dealing ranged damage. No draw/wind-up
    /// phase like `Ranged` (that's specifically documented for bow users) - casts immediately
    /// once off cooldown and in `range`.
    FishingRod { cast_speed_bps: f64, cooldown_ticks: u32, range: f64 },
}

/// Which projectile/visual a `HybridMeleeRanged` (or a future dedicated `Ranged`) archetype's
/// ranged half uses - a wither skull (no bow-hold, see `try_ranged`'s `uses_bow`) or a real
/// arrow (does hold/raise a bow, same as `AttackModule::Ranged`'s bow users).
#[derive(Clone, Copy)]
pub enum RangedProjectile {
    Arrow,
    WitherSkull,
}

/// Ticks the swing lasts once triggered - purely a visual arm-pose duration.
const MELEE_SWING_TICKS: u16 = 6;
/// Default ticks between melee swings (~1 second at 20 TPS) for archetypes that don't set
/// `AttackModule::Melee`'s `cooldown_ticks` to something more specific. Not specified precisely
/// in the design notes for BASIC_ZOMBIE; a reasonable placeholder pending real tuning.
pub const MELEE_COOLDOWN_TICKS: u16 = 20;
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
        AttackModule::Melee { range, cooldown_ticks } => {
            state.bow_draw_ticks = 0; // pure-melee archetypes never draw - keep the field inert
            try_melee(world, entity, range, cooldown_ticks, target_pos);
        }
        AttackModule::Ranged { speed_bps, cooldown_ticks, range } => {
            try_ranged(world, entity, state, speed_bps, cooldown_ticks, range, target_pos, true, fire_arrow_at_player);
        }
        AttackModule::HybridMeleeRanged { melee_range, ranged_speed_bps, ranged_cooldown_ticks, ranged_range, projectile } => {
            if entity.position.distance_to(&target_pos) < melee_range {
                state.bow_draw_ticks = 0; // switched to melee range - cancel any in-progress nock
                try_melee(world, entity, melee_range, MELEE_COOLDOWN_TICKS, target_pos);
            } else {
                match projectile {
                    RangedProjectile::Arrow => try_ranged(world, entity, state, ranged_speed_bps, ranged_cooldown_ticks, ranged_range, target_pos, true, fire_arrow_at_player),
                    RangedProjectile::WitherSkull => try_ranged(world, entity, state, ranged_speed_bps, ranged_cooldown_ticks, ranged_range, target_pos, false, fire_wither_skull_at_player),
                }
            }
        }
        AttackModule::FishingRod { cast_speed_bps, cooldown_ticks, range } => {
            try_fishing_rod(world, entity, state, cast_speed_bps, cooldown_ticks, range, target_pos);
        }
    }
}

fn try_fishing_rod(
    world: &mut World,
    entity: &mut Entity,
    state: &mut MobAiState,
    speed_bps: f64,
    cooldown_ticks: u32,
    range: f64,
    target_pos: DVec3,
) {
    if entity.position.distance_to(&target_pos) > range {
        return;
    }
    if !state.attack_cooldowns.primary.is_ready() {
        return;
    }

    let origin = DVec3::new(entity.position.x, entity.position.y + SHOOTER_HAND_HEIGHT, entity.position.z);
    let aim_point = DVec3::new(target_pos.x, target_pos.y + TARGET_AIM_HEIGHT, target_pos.z);
    let shooter_id = entity.id;

    if fire_fishing_hook_at_player(world, origin, aim_point, speed_bps, shooter_id).is_ok() {
        let jitter = rand::rng().random_range(-RANGED_COOLDOWN_JITTER_TICKS..=RANGED_COOLDOWN_JITTER_TICKS);
        let jittered_cooldown = (cooldown_ticks as i32 + jitter).max(1) as u32;
        state.attack_cooldowns.primary.trigger(jittered_cooldown);

        for player in world.players.values_mut() {
            player.write_packet(&SoundEffect {
                sound: Sounds::Bow.id(), // no dedicated fishing-rod-cast sound wired up yet
                pos_x: origin.x,
                pos_y: origin.y,
                pos_z: origin.z,
                volume: 1.0,
                pitch: 1.0,
            });
        }
    }
}

fn try_melee(world: &mut World, entity: &mut Entity, range: f64, cooldown_ticks: u16, target_pos: DVec3) {
    if entity.position.distance_to(&target_pos) > range {
        return;
    }

    let entity_id = entity.id;
    let Some(cooldown) = world.get_attack_cooldown(entity_id) else { return };
    if cooldown.ticks > 0 {
        return;
    }

    world.set_attack_cooldown(entity_id, AttackCooldown { ticks: cooldown_ticks });
    if let Some(combat_state) = world.get_combat_state_mut(entity_id) {
        combat_state.aggressive = true;
        combat_state.swing_ticks = MELEE_SWING_TICKS;
    }

    // The actual visible swing - a vanilla `Animation` (swing main arm) packet. Without this, a
    // player-model melee archetype (Shadow Assassin, Crypt Dreadlord/Undead, King Midas) never
    // visibly swings at all: the Zombie-only pose flip just below is a no-op for any of them
    // (wrong variant), so the attack cooldown/combat-state bookkeeping below was firing with
    // nothing to show for it - confirmed as a real, reported gap, not just a Zombie-model
    // nicety. This IS a real swing/punch motion, unlike `set_drawing_bow` below - a bow draw
    // uses the persistent "using item" pose instead, not this.
    broadcast_swing(world, entity_id);

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

/// Ticks a ranged attacker spends visibly winding up before actually releasing its shot -
/// "raise their bows when preparing to shoot ... visually draw their bows before firing,
/// rather than shooting instantly" - real vanilla `EntityAIArrowAttack`/wither-skull-style
/// windups aren't instant either. ~0.6s at 20 TPS.
const BOW_DRAW_TICKS: u32 = 12;

/// Sets/clears the persistent vanilla "using item" pose (see `EntityMetadata::is_using_item`'s
/// own doc comment) for the duration of a bow draw. Confirmed by explicit report: an earlier
/// version of this used a repeated `Animation` (swing) packet instead, on the theory that
/// Skeleton's metadata table has no dedicated "drawing" bit - that's true, but the fix isn't a
/// swing (the melee-punch animation) either; `is_using_item` is a base `Entity` flag, not a
/// player-only one, so it applies the same way to a mob-model archetype (Skeleton family) as a
/// player-model one.
pub(crate) fn set_drawing_bow(entity: &mut Entity, world: &mut World, drawing: bool) {
    entity.metadata.is_using_item = drawing;
    world.send_metadata_update(entity.id);
}

fn try_ranged(
    world: &mut World,
    entity: &mut Entity,
    state: &mut MobAiState,
    speed_bps: f64,
    cooldown_ticks: u32,
    range: f64,
    target_pos: DVec3,
    uses_bow: bool,
    fire: impl FnOnce(&mut World, DVec3, DVec3, f64) -> anyhow::Result<crate::server::entity::entity::EntityId>,
) {
    // Out of engagement range - "lower the bow": cancel any in-progress draw rather than firing
    // from further away than this archetype's documented preferred distance.
    if entity.position.distance_to(&target_pos) > range {
        if uses_bow && state.bow_draw_ticks > 0 {
            lower_bow(world, entity.id, state.archetype);
            set_drawing_bow(entity, world, false);
        }
        state.bow_draw_ticks = 0;
        return;
    }

    if state.bow_draw_ticks > 0 {
        state.bow_draw_ticks -= 1;
        if state.bow_draw_ticks == 0 {
            let origin = DVec3::new(entity.position.x, entity.position.y + SHOOTER_HAND_HEIGHT, entity.position.z);
            let aim_point = DVec3::new(target_pos.x, target_pos.y + TARGET_AIM_HEIGHT, target_pos.z);

            if uses_bow {
                lower_bow(world, entity.id, state.archetype);
                set_drawing_bow(entity, world, false);
            }

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
        return;
    }

    if !state.attack_cooldowns.primary.is_ready() {
        return;
    }

    // Off cooldown and in range - start raising/drawing the bow. The actual shot fires once
    // `bow_draw_ticks` counts down to 0 above, on some later tick.
    state.bow_draw_ticks = BOW_DRAW_TICKS;
    if uses_bow {
        raise_bow(world, entity.id, state.archetype);
        set_drawing_bow(entity, world, true);
    }
}

/// Swaps a real player-model bow-user archetype's mainhand item to a plain Bow for the
/// duration of its draw/nock windup - a `spawn_as_npc` mob's held item is real `EntityEquipment`
/// state (unlike a vanilla Skeleton, whose inherent model already shows its own bow-draw pose
/// with no item-slot involvement at all - see this function's `spawn_as_npc()` gate), so without
/// this a player-model archetype visibly swings whatever's actually in its hand (its flavor
/// melee weapon, e.g. Lost Adventurer's "Aspect of the Dragons") while firing arrows - looks
/// like punching, not shooting. Doesn't touch the entity's *stored* `world.entity_equipment`
/// (its permanent, canonical held item, restored on chunk resync) - only broadcasts a temporary
/// visual override to already-connected players, itself restored by `lower_bow` once the shot
/// resolves or the draw is cancelled.
pub(crate) fn raise_bow(world: &mut World, entity_id: crate::server::entity::entity::EntityId, archetype: crate::server::entity::dungeon_mobs::mob_type::DungeonMobType) {
    if !archetype.spawn_as_npc() {
        return;
    }
    use crate::net::protocol::play::clientbound::EntityEquipment;
    use crate::net::var_int::VarInt;
    use crate::server::items::item_stack::ItemStack;
    for player in world.players.values_mut() {
        player.write_packet(&EntityEquipment {
            entity_id: VarInt(entity_id),
            item_slot: 0,
            item_stack: Some(ItemStack::new(261)), // bow
        });
    }
}

/// Restores whatever's actually stored as this entity's real mainhand item - the other half of
/// `raise_bow`, see its doc comment.
pub(crate) fn lower_bow(world: &mut World, entity_id: crate::server::entity::entity::EntityId, archetype: crate::server::entity::dungeon_mobs::mob_type::DungeonMobType) {
    if !archetype.spawn_as_npc() {
        return;
    }
    use crate::net::protocol::play::clientbound::EntityEquipment;
    use crate::net::var_int::VarInt;
    let original = world.entity_equipment.get(&entity_id).and_then(|eq| eq.main_hand.clone());
    for player in world.players.values_mut() {
        player.write_packet(&EntityEquipment {
            entity_id: VarInt(entity_id),
            item_slot: 0,
            item_stack: original.clone(),
        });
    }
}

/// Broadcasts a vanilla "swing main arm" `Animation` packet for `entity_id` to every connected
/// player - see the packet's own doc comment for why this (not a metadata pose bit) is the real
/// vanilla mechanism for a mob's visible bow-draw/nock motion.
pub(crate) fn broadcast_swing(world: &mut World, entity_id: crate::server::entity::entity::EntityId) {
    use crate::net::protocol::play::clientbound::Animation;
    use crate::net::var_int::VarInt;
    for player in world.players.values_mut() {
        player.write_packet(&Animation { entity_id: VarInt(entity_id), animation_id: 0 });
    }
}

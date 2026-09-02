//! Modular dungeon-mob AI: a mob archetype is *composed* from reusable pieces (perception,
//! aggro propagation, leash/return-to-spawn, green-room disengagement, movement style,
//! attack modules, cooldowns) rather than each mob getting its own bespoke AI class, and
//! rather than falling back to plain vanilla mob AI. See `ai/profile.rs` for where an
//! archetype picks which reusable pieces it's built from.
//!
//! Per-entity mutable AI state lives in one `World.entity_mob_ai: HashMap<EntityId,
//! MobAiState>` map (see `state.rs` for why one map, not one per component) rather than
//! inside this module's `EntityImpl` - that keeps `DungeonMobAiImpl` and
//! `DungeonPlayerMobImpl` (the two `EntityImpl`s that drive dungeon mobs) both stateless
//! wrappers that can share the exact same `run_mob_ai` pipeline.

pub mod aggro;
pub mod attack;
pub mod combat;
pub mod cooldown;
pub mod green_room;
pub mod leash;
pub mod movement;
pub mod pathfinding;
pub mod perception;
pub mod physics;
pub mod profile;
pub mod projectile;
pub mod state;

use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::server::entity::dungeon_mobs::ai::profile::{profile_for, IDLE_ACTIVATION_RANGE};
use crate::server::entity::dungeon_mobs::ai::state::ActivationState;
use crate::server::entity::dungeon_mobs::mob_type::MobBaseKind;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::EntityVariant;
use crate::server::player::player::Player;

/// `EntityImpl` for every dungeon-mob archetype that isn't spawned as a real player-model NPC
/// (see `spawner.rs`'s `spawn_as_npc` branch, which uses `DungeonPlayerMobImpl` instead but
/// shares this same `run_mob_ai` pipeline).
pub struct DungeonMobAiImpl;

impl EntityImpl for DungeonMobAiImpl {
    fn tick(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        run_mob_ai(entity, packet_buffer);
    }

    fn interact(&mut self, entity: &mut Entity, player: &mut Player, action: &EntityInteractionType) -> bool {
        if *action == EntityInteractionType::Attack {
            if !combat::apply_king_midas_hit(entity, player) {
                combat::apply_lethal_hit(entity, player);
            }
            aggro::on_mob_attacked(entity, player.client_id);
        }
        false
    }
}

/// Base walk speed (blocks/sec) used for approach/maintain-distance/leash-return movement.
/// Not specified precisely anywhere in the design notes; a reasonable vanilla-zombie-ish
/// placeholder (1.5x the original placeholder value per user feedback), scaled per archetype
/// by `AiProfile.speed_multiplier`.
const BASE_WALK_SPEED_BPS: f64 = 3.75;
/// Faster forward speed used only for the one-shot "first sight" nudge.
const FIRST_SIGHT_SPEED_BPS: f64 = 5.25;
/// Slower speed used for idle wandering near spawn.
const IDLE_WALK_SPEED_BPS: f64 = 2.25;
/// How close idle wandering needs to get to its target before picking a new one.
const IDLE_WANDER_ARRIVAL_DISTANCE: f64 = 0.3;
/// Vanilla's real sprint speed (~0.2806 blocks/tick * 20 TPS). Base for humanoid NPC-model
/// dungeon mobs' (`archetype.spawn_as_npc()`) combat movement, which per request runs at
/// `HUMANOID_SPRINT_MULTIPLIER`x a real player's sprint rather than `BASE_WALK_SPEED_BPS` -
/// zombie-model archetypes are unaffected, only the real player-model NPCs (Crypt Dreadlord/
/// Souleater/Undead, the Adventurers, Zombie Commander, King Midas).
const PLAYER_SPRINT_SPEED_BPS: f64 = 5.612;
const HUMANOID_SPRINT_MULTIPLIER: f64 = 1.5;
/// How often (ticks) an in-combat mob re-checks line of sight to its target - the check itself
/// is a block-sampled raycast (`perception::has_los_to_player`), not free, so it isn't run every
/// tick.
const LOS_CHECK_INTERVAL_TICKS: u32 = 10;
/// How long (ticks) without confirmed LOS before a mob gives up on its target and heads home -
/// "relatively oblivious", but not zero-memory: a target that's out of sight only briefly (e.g.
/// stepping behind a corner and back) doesn't instantly break the chase.
const LOS_FORGET_TICKS: u32 = 100;
/// Multiplier on `BASE_WALK_SPEED_BPS` used for the lasso return trip once a mob has committed
/// to it (leash distance exceeded, green room, or gave up per `LOS_FORGET_TICKS`) - "insta lasso
/// back... at 2x walking speed", not the normal combat-approach pace.
const LASSO_RETURN_SPEED_MULTIPLIER: f64 = 2.0;

/// The shared AI think-and-act pipeline, run once per tick per dungeon mob. Shared by
/// `DungeonMobAiImpl::tick` and `DungeonPlayerMobImpl::tick` so NPC-model archetypes get the
/// exact same behavior as mob-model ones.
pub fn run_mob_ai(entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
    let entity_id = entity.id;
    let world = entity.world_mut();

    let Some(mut state) = world.entity_mob_ai.get(&entity_id).copied() else { return };
    let (width, height) = physics::size_for(state.archetype.base_kind());

    // Basic physics applies to every dungeon mob regardless of whether its AI behavior is
    // implemented yet - falling and not clipping through walls/players/each other is a
    // baseline physical property, not a per-archetype behavior choice.
    physics::apply_gravity(entity, world, width, height);
    let (push_x, push_z) = physics::separation_push(world, entity_id, entity.position, width);
    if push_x != 0.0 || push_z != 0.0 {
        physics::move_horizontal(entity, world, width, height, push_x, push_z, false);
    }

    let profile = profile_for(state.archetype);
    if !profile.implemented {
        // Archetype has no real behavior yet (Phase 3+) - leave it exactly as it behaves
        // today (inert) rather than guessing at unverified Hypixel mechanics.
        return;
    }

    let mob_pos = entity.position;

    // --- Activation: idle-activation range is distinct from (and larger than) combat vision ---
    let nearest_player_distance = world.players.values()
        .map(|player| player.position.distance_to(&mob_pos))
        .fold(f64::INFINITY, f64::min);

    // Perception is skipped entirely while committed to a lasso return (`state.leashed_out`) -
    // per request, a mob at its leash limit shouldn't "fight the active mode" by re-engaging
    // combat partway home the instant a target comes back in view; it commits to the return
    // trip (see the leashed_out movement branch below) and only starts looking for a target
    // again once it's actually back (`leash::tick_leash` clears `leashed_out` on arrival).
    if state.target.is_none() && !state.leashed_out {
        if nearest_player_distance > IDLE_ACTIVATION_RANGE {
            state.activation = ActivationState::Dormant;
            state.idle_wander_target = None;
        } else {
            if state.activation == ActivationState::Dormant {
                state.activation = ActivationState::Idle;
            }

            // --- Perception: try to acquire a combat target, then propagate locally ---
            if let Some(found) = perception::find_vision_target(world, mob_pos, profile.vision_range, profile.requires_los) {
                state.target = Some(found);
                state.activation = ActivationState::Combat;
                state.first_sight_pending = true;
                state.leashed_out = false;
                state.bow_draw_ticks = 0; // fresh target - don't resume a stale draw from before
                state.ticks_since_los = 0; // fresh target - definitely just saw them (acquisition itself required LOS)
                state.los_check_cooldown.trigger(LOS_CHECK_INTERVAL_TICKS);
                aggro::acquire_or_propagate_target(world, mob_pos, found, profile.aggro_propagation_radius);
            }
        }
    }

    // --- Green-room disengagement forces the same return-to-spawn path as an exceeded leash;
    // lost-line-of-sight does too, but only after `LOS_FORGET_TICKS` of not seeing the target
    // ("relatively oblivious", not zero-memory) - re-checked at most once every
    // `LOS_CHECK_INTERVAL_TICKS` (the raycast itself isn't free) rather than every tick.
    if let Some(target) = state.target {
        state.los_check_cooldown.tick();
        if profile.requires_los && state.los_check_cooldown.is_ready() {
            state.los_check_cooldown.trigger(LOS_CHECK_INTERVAL_TICKS);
            let has_los = world.players.get(&target).map(|player| player.position)
                .is_some_and(|pos| perception::has_los_to_player(world, mob_pos, pos));
            if has_los {
                state.ticks_since_los = 0;
            } else {
                state.ticks_since_los = state.ticks_since_los.saturating_add(LOS_CHECK_INTERVAL_TICKS);
            }
        }

        if green_room::target_in_green_room(world, target) || state.ticks_since_los >= LOS_FORGET_TICKS {
            leash::disengage(&mut state);
        }
    }
    leash::tick_leash(&mut state, mob_pos, profile.leash_distance);

    // --- Persistent visual state driven by activation, not the moment-to-moment attack/swing
    // cycle: a Zombie-model mob's raised-arms "detected you" pose, and a humanoid NPC-model
    // mob's sprint animation - both should reflect "currently in combat", not flicker with
    // individual swings/shots. Only resent on an actual change (see `state.arms_raised`/
    // `sprinting`'s doc comments), not every tick.
    let active = state.activation == ActivationState::Combat;
    let mut pose_changed = false;

    if state.archetype.base_kind() == MobBaseKind::Zombie && active != state.arms_raised {
        state.arms_raised = active;
        if let EntityVariant::Zombie { is_child, is_villager, is_converting, .. } = entity.metadata.variant {
            entity.metadata.variant = EntityVariant::Zombie { is_child, is_villager, is_converting, is_attacking: active };
            pose_changed = true;
        }
    }

    let should_sprint = active && state.archetype.spawn_as_npc();
    if should_sprint != state.sprinting {
        state.sprinting = should_sprint;
        entity.metadata.is_sprinting = should_sprint;
        pose_changed = true;
    }

    if pose_changed {
        world.send_metadata_update(entity_id);
    }

    // --- Movement + attack ---
    if state.leashed_out {
        // "Insta lasso back... at 2x walking speed" - a committed return trip is quick and
        // uninterruptible (see the perception-skip above), not the normal combat-approach pace.
        movement::steer_toward(entity, world, width, height, state.spawn_origin, BASE_WALK_SPEED_BPS * LASSO_RETURN_SPEED_MULTIPLIER * profile.speed_multiplier, false);
    } else if let Some(target) = state.target {
        let Some(target_pos) = world.players.get(&target).map(|player| player.position) else {
            // Target disconnected mid-fight - just drop it, no leash walk-back needed.
            state.target = None;
            state.activation = ActivationState::Idle;
            world.entity_mob_ai.insert(entity_id, state);
            return;
        };

        if state.first_sight_pending {
            movement::steer_toward(entity, world, width, height, target_pos, FIRST_SIGHT_SPEED_BPS * profile.speed_multiplier, false);
            state.first_sight_pending = false;
        } else {
            // Humanoid NPC-model archetypes sprint (see `should_sprint` above) rather than
            // walking at `BASE_WALK_SPEED_BPS` while actively in combat.
            let combat_speed_bps = if state.archetype.spawn_as_npc() {
                PLAYER_SPRINT_SPEED_BPS * HUMANOID_SPRINT_MULTIPLIER * profile.speed_multiplier
            } else {
                BASE_WALK_SPEED_BPS * profile.speed_multiplier
            };
            let (steer_pos, on_path) = pathfinding::next_steer_point(world, entity_id, mob_pos, target_pos, width, height);
            movement::apply_movement_style(entity, world, width, height, profile.movement, target_pos, steer_pos, combat_speed_bps, on_path);
        }

        // Only snap to look straight at the real target once actually within striking range of
        // it (matches `try_attack`'s own melee range check just below, so "facing the target"
        // and "able to hit the target" line up) - otherwise `steer_toward`/`apply_movement_style`
        // above already set yaw+pitch toward wherever the mob is actually walking (the path
        // waypoint, or straight at the target if that's also the walking direction), which is
        // what a mob crossing the room toward a staircase should look like doing, not dead-
        // staring at the player the whole way there. The `MaintainDistance`/`HybridMeleeRanged`
        // "hold position and face the target while shooting" branch inside `apply_movement_style`
        // already handles its own case correctly without this - this only needs to cover melee
        // closing the last stretch in.
        let in_strike_range = match profile.attack {
            attack::AttackModule::Melee { range } => entity.position.distance_to(&target_pos) <= range,
            attack::AttackModule::HybridMeleeRanged { melee_range, .. } => entity.position.distance_to(&target_pos) <= melee_range,
            attack::AttackModule::Ranged { .. } | attack::AttackModule::FishingRod { .. } => false,
        };
        if in_strike_range {
            movement::face_toward(entity, target_pos);
        }

        state.attack_cooldowns.primary.tick();
        state.attack_cooldowns.secondary.tick();
        attack::try_attack(world, entity, &mut state, profile.attack, target_pos);
    } else if state.activation == ActivationState::Idle {
        // Passive mobs don't track nearby players at all - they just look straight forward
        // (their original spawn-facing direction) except while actually mid-wander-step, where
        // `steer_toward` naturally turns to face the wander direction instead.
        state.idle_wander_cooldown.tick();
        if state.idle_wander_target.is_none() && state.idle_wander_cooldown.is_ready() {
            state.idle_wander_target = Some(movement::pick_idle_wander_target(state.spawn_origin));
            state.idle_wander_cooldown.trigger(movement::random_idle_wander_interval());
        }

        if let Some(wander_target) = state.idle_wander_target {
            if mob_pos.distance_to(&wander_target) < IDLE_WANDER_ARRIVAL_DISTANCE {
                state.idle_wander_target = None;
            } else {
                movement::steer_toward(entity, world, width, height, wander_target, IDLE_WALK_SPEED_BPS, false);
            }
        } else {
            entity.yaw = state.spawn_yaw;
            entity.pitch = 0.0;
        }
    }

    world.entity_mob_ai.insert(entity_id, state);
}

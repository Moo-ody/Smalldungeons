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
pub mod perception;
pub mod physics;
pub mod profile;
pub mod projectile;
pub mod state;

use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::server::entity::dungeon_mobs::ai::profile::{profile_for, IDLE_ACTIVATION_RANGE};
use crate::server::entity::dungeon_mobs::ai::state::ActivationState;
use crate::server::entity::entity::{Entity, EntityImpl};
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
/// How close a player has to be for an idle (not currently in combat) mob to turn and face them.
/// Deliberately much tighter than `IDLE_ACTIVATION_RANGE`/any archetype's `vision_range` - those
/// gate actually noticing/aggroing a player, which already keeps a mob facing its target once
/// combat starts (see `movement::apply_movement_style`); this is a separate, purely cosmetic
/// "personal space" reaction for a player standing right next to an otherwise-idle mob.
const IDLE_FACE_PLAYER_RANGE: f64 = 8.0;
/// How close idle wandering needs to get to its target before picking a new one.
const IDLE_WANDER_ARRIVAL_DISTANCE: f64 = 0.3;

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
        physics::move_horizontal(entity, world, width, height, push_x, push_z);
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

    // Perception runs even while walking back from an exceeded leash/green-room disengage -
    // if the target comes back within range, combat resumes instead of finishing the walk home.
    if state.target.is_none() {
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
                aggro::acquire_or_propagate_target(world, mob_pos, found, profile.aggro_propagation_radius);
            }
        }
    }

    // --- Green-room disengagement forces the same return-to-spawn path as an exceeded leash ---
    if let Some(target) = state.target {
        if green_room::target_in_green_room(world, target) {
            leash::disengage(&mut state);
        }
    }
    leash::tick_leash(&mut state, mob_pos, profile.leash_distance);

    // --- Movement + attack ---
    if state.leashed_out {
        movement::steer_toward(entity, world, width, height, state.spawn_origin, BASE_WALK_SPEED_BPS * profile.speed_multiplier);
    } else if let Some(target) = state.target {
        let Some(target_pos) = world.players.get(&target).map(|player| player.position) else {
            // Target disconnected mid-fight - just drop it, no leash walk-back needed.
            state.target = None;
            state.activation = ActivationState::Idle;
            world.entity_mob_ai.insert(entity_id, state);
            return;
        };

        if state.first_sight_pending {
            movement::steer_toward(entity, world, width, height, target_pos, FIRST_SIGHT_SPEED_BPS * profile.speed_multiplier);
            state.first_sight_pending = false;
        } else {
            movement::apply_movement_style(entity, world, width, height, profile.movement, target_pos, BASE_WALK_SPEED_BPS * profile.speed_multiplier);
        }

        state.attack_cooldowns.primary.tick();
        state.attack_cooldowns.secondary.tick();
        attack::try_attack(world, entity, &mut state, profile.attack, target_pos);
    } else if state.activation == ActivationState::Idle {
        let nearest_player_pos = world.players.values()
            .map(|player| (player.position, player.position.distance_to(&mob_pos)))
            .filter(|(_, dist)| *dist <= IDLE_FACE_PLAYER_RANGE)
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(pos, _)| pos);

        if let Some(player_pos) = nearest_player_pos {
            // A player standing right next to an idle mob takes priority over wandering - stop
            // and look at them instead of continuing to wander while also fighting over yaw.
            movement::face_toward(entity, player_pos);
        } else {
            state.idle_wander_cooldown.tick();
            if state.idle_wander_target.is_none() && state.idle_wander_cooldown.is_ready() {
                state.idle_wander_target = Some(movement::pick_idle_wander_target(state.spawn_origin));
                state.idle_wander_cooldown.trigger(movement::random_idle_wander_interval());
            }

            if let Some(wander_target) = state.idle_wander_target {
                if mob_pos.distance_to(&wander_target) < IDLE_WANDER_ARRIVAL_DISTANCE {
                    state.idle_wander_target = None;
                } else {
                    movement::steer_toward(entity, world, width, height, wander_target, IDLE_WALK_SPEED_BPS);
                }
            }
        }
    }

    world.entity_mob_ai.insert(entity_id, state);
}

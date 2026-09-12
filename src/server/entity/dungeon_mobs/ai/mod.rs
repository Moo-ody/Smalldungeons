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
use crate::server::entity::dungeon_mobs::ai::cooldown::Cooldown;
use crate::server::entity::dungeon_mobs::ai::movement::MovementStyle;
use crate::server::entity::dungeon_mobs::ai::profile::{profile_for, IDLE_ACTIVATION_RANGE};
use crate::server::entity::dungeon_mobs::ai::state::ActivationState;
use crate::server::entity::dungeon_mobs::mob_type::MobBaseKind;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::EntityVariant;
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use rand::Rng;

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
            combat::on_player_damaged_mob(entity.world_mut(), entity.id);
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
/// Both the distance behind the target a `teleport_ambush` archetype's teleport lands at, and
/// the standoff distance its "plain orbit" phase (see the weave-cycle constants below) settles
/// into and returns to after each weave-out - "up close and personal" per explicit follow-up
/// correction (an earlier 3.0, just past `AttackModule::Melee`'s own 2.5 `range`, read as
/// orbiting too far out - this sits comfortably inside melee range instead).
const ORBIT_RADIUS: f64 = 1.8;
/// How often (ticks) a `teleport_ambush` archetype re-rolls whether to start a bow volley while
/// orbiting - it now attacks continuously every tick while in range instead (see the module doc
/// comment's "way more offensive" correction), so this only paces the volley-chance roll.
const WEAVE_CYCLE_TICKS: u32 = 10;
/// Chance, at each volley-roll point, of a bow volley instead of continuing to melee - "rare"
/// per explicit follow-up correction (originally 0.25, judged too frequent).
const VOLLEY_CHANCE: f64 = 0.1;
/// Tangential orbit speed multiplier (on `combat_speed_bps`) for the "plain orbit" phase - kept
/// well below full sprint per explicit request ("shouldn't rotate as much when up close, should
/// be trying to focus on meleeing") so the circling reads as a controlled prowl rather than a
/// fast spin, and he settles into landing weave-in hits more than visibly orbiting.
const ORBIT_SPEED_MULTIPLIER: f64 = 0.35;
/// Arrows fired per volley, and ticks between each - both per explicit request ("5 times at 10
/// ticks per arrow").
const VOLLEY_ARROW_COUNT: u8 = 5;
const VOLLEY_SHOT_INTERVAL_TICKS: u32 = 10;
/// Arrow speed for a `teleport_ambush` archetype's volley - no documented value, matches the
/// existing bow-user archetypes' own speed (`profile.rs`'s Lost Adventurer/Skeleton entries).
const VOLLEY_ARROW_SPEED_BPS: f64 = 24.0;
/// How often (ticks) an `AiProfile::hops_while_approaching` archetype hops while charging in -
/// no documented value, tuned to read as a bouncy jog rather than a single-jump novelty or a
/// dizzying blur.
const HOP_INTERVAL_TICKS: u32 = 12;
/// Ticks of no LOS to any player and no damage dealt in either direction before a released
/// (`MobAiState::damaged_once`) `AiProfile::room_confined` archetype teleports back to its home
/// room - "100 consecutive ticks... no line of sight and no damage dealt in either direction",
/// per explicit request.
const NO_INTERACTION_RETURN_TICKS: u32 = 100;

/// Whether `pos` falls within `room_index`'s own world X/Z bounds (Y ignored) - used by
/// `AiProfile::room_confined` to decide whether a target is actually reachable without leaving
/// the archetype's home room. `true` if the room can't be looked up at all (fails open rather
/// than permanently trapping a mob whose room somehow went missing).
fn target_in_room(world: &World, room_index: usize, pos: DVec3) -> bool {
    let Some(room) = world.server_mut().dungeon.rooms.get(room_index) else { return true };
    let (min, max) = room.get_world_bounds();
    pos.x >= min.x as f64 && pos.x <= (max.x + 1) as f64
        && pos.z >= min.z as f64 && pos.z <= (max.z + 1) as f64
}

/// Fires one volley arrow from `entity` at `target_pos`, plus the usual bow-loose sound - used
/// only by a `teleport_ambush` archetype's stationary bow-volley phase (see the module doc
/// comment above), not the generic `attack::try_ranged` draw/fire cycle (a volley has no
/// per-shot windup, just a fixed cadence - see `VOLLEY_SHOT_INTERVAL_TICKS`).
fn fire_volley_arrow(world: &mut World, entity: &Entity, target_pos: DVec3) {
    let origin = DVec3::new(entity.position.x, entity.position.y + 1.4, entity.position.z);
    let aim_point = DVec3::new(target_pos.x, target_pos.y + 1.5, target_pos.z);
    if projectile::fire_arrow_at_player(world, origin, aim_point, VOLLEY_ARROW_SPEED_BPS).is_ok() {
        use crate::net::protocol::play::clientbound::SoundEffect;
        use crate::server::utils::sounds::Sounds;
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

/// Repositions `entity` directly behind `target_pos`, relative to `target_yaw` (the target's own
/// facing) - the point an observer standing at `target_pos` and looking in `target_yaw` would
/// have directly behind them. Doesn't set yaw/pitch itself - every caller immediately follows
/// this with `movement::face_toward(entity, target_pos)`, which (from directly behind, looking
/// back at the target) naturally produces the same "facing their back" orientation `target_yaw`
/// itself would, so there's no need to compute it twice.
fn teleport_behind(entity: &mut Entity, target_pos: DVec3, target_yaw: f32) {
    let yaw_rad = (target_yaw as f64).to_radians();
    // Forward direction the target is facing, in this codebase's yaw convention (0 = south/+Z,
    // 90 = west/-X - see `movement::yaw_towards`) - "behind" is the opposite of this.
    let forward_x = -yaw_rad.sin();
    let forward_z = yaw_rad.cos();
    entity.position = DVec3::new(
        target_pos.x - forward_x * ORBIT_RADIUS,
        target_pos.y,
        target_pos.z - forward_z * ORBIT_RADIUS,
    );
}

/// The shared AI think-and-act pipeline, run once per tick per dungeon mob. Shared by
/// `DungeonMobAiImpl::tick` and `DungeonPlayerMobImpl::tick` so NPC-model archetypes get the
/// exact same behavior as mob-model ones.
pub fn run_mob_ai(entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
    let entity_id = entity.id;
    let world = entity.world_mut();

    // Touching lava (Lava Ravine/Lava Pit's terrain hazard, or falling into it) instantly kills
    // a dungeon mob, per explicit request - a baseline environmental hazard, not a per-archetype
    // behavior choice, so this runs unconditionally before anything else here (including the
    // `!profile.implemented` early-return below - a mob with no real AI yet still dies in lava).
    // Routed through the same `combat::kill_mob` every other death goes through, so it grants a
    // door key / clears the room the same way a normal kill does if this was the last starred mob.
    if physics::is_in_lava(world, entity.position) {
        combat::kill_mob(world, entity_id);
        return;
    }

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

    // --- Yellow-room miniboss containment (released half): once a player has actually landed a
    // hit (`damaged_once`), auto-teleport back to its home room after 100 ticks with no LOS to
    // any player and no damage dealt in either direction - see `AiProfile::room_confined`'s own
    // doc comment for the full mechanic (the pre-release half - refusing to engage a target
    // outside the room at all - lives further down, where a target/movement decision is
    // actually made). A fresh, unthrottled LOS scan (not the throttled per-target
    // `ticks_since_los`) since this has to keep working even with no target at all.
    if profile.room_confined && state.damaged_once {
        let has_los = perception::find_vision_target(world, mob_pos, profile.vision_range, true).is_some();
        if has_los {
            state.no_interaction_ticks = 0;
        } else {
            state.no_interaction_ticks += 1;
        }
        if state.no_interaction_ticks >= NO_INTERACTION_RETURN_TICKS {
            entity.position = state.spawn_origin;
            entity.velocity = DVec3::ZERO;
            state.no_interaction_ticks = 0;
            state.target = None;
            state.leashed_out = false;
            state.activation = ActivationState::Idle;
            world.entity_mob_ai.insert(entity_id, state);
            return;
        }
    }

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
        // A mob pre-spawned dormant into a room the player hasn't actually entered yet (see
        // `MobAiState::room_entered`'s own doc comment / `Dungeon::tick`'s adjacent-room pre-
        // spawn check) stays fully asleep - visible, but no perception/idle-wander - regardless
        // of how close a player standing in a NEIGHBOURING room happens to be to the shared
        // door. `room_entered` is a plain flag pushed into this state directly by `Dungeon::tick`
        // the tick the room is actually entered (not re-derived here via `world.server_mut()`,
        // which would need a real `Server`/`Dungeon` behind every single mob tick - see
        // `perf_bench.rs`'s own doc comment on why an idle mob must never need that).
        if !state.room_entered || nearest_player_distance > IDLE_ACTIVATION_RANGE {
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
                // Shadow Assassin (or any future `teleport_ambush` archetype): freeze invisible
                // for the ambush's stun window instead of the normal first-sight nudge - see
                // the dedicated branch in the movement/attack section below.
                if let Some(ambush) = profile.teleport_ambush {
                    state.stun_ticks = ambush.stun_ticks;
                    state.teleport_cooldown = Cooldown::ready();
                }
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
        let Some((target_pos, target_yaw)) = world.players.get(&target).map(|player| (player.position, player.yaw)) else {
            // Target disconnected mid-fight - just drop it, no leash walk-back needed.
            state.target = None;
            state.activation = ActivationState::Idle;
            world.entity_mob_ai.insert(entity_id, state);
            return;
        };

        // Humanoid NPC-model archetypes sprint (see `should_sprint` above) rather than
        // walking at `BASE_WALK_SPEED_BPS` while actively in combat.
        let combat_speed_bps = if state.archetype.spawn_as_npc() {
            PLAYER_SPRINT_SPEED_BPS * HUMANOID_SPRINT_MULTIPLIER * profile.speed_multiplier
        } else {
            BASE_WALK_SPEED_BPS * profile.speed_multiplier
        };

        // --- Yellow-room miniboss containment (pre-release half): refuses to engage a target
        // standing outside its own home room at all - no movement, no attack - until a player
        // actually lands a hit (see the containment block up near `mob_pos` for the released
        // half). It was already chasing normally right up until the target stepped out, so this
        // naturally reads as camping the doorway rather than needing its own clamped-movement
        // logic - it simply stops advancing the instant the target is no longer reachable.
        if profile.room_confined && !state.damaged_once && !target_in_room(world, state.room_index, target_pos) {
            movement::face_toward(entity, target_pos);
            world.entity_mob_ai.insert(entity_id, state);
            return;
        }

        if let Some(ambush) = profile.teleport_ambush {
            // Shadow Assassin's documented ambush pattern (see `TeleportAmbush`'s doc comment) -
            // entirely bypasses the generic first-sight-nudge/pathfinding-approach flow below:
            // it never walks toward its target at all, only teleports directly behind it, then
            // (once actually engaged) orbits it in one fixed direction, periodically darting in
            // to melee or stopping entirely for a bow volley - see below.
            state.first_sight_pending = false;

            if state.stun_ticks > 0 {
                state.stun_ticks -= 1;
                world.entity_mob_ai.insert(entity_id, state);
                return;
            }

            let distance = entity.position.distance_to(&target_pos);
            // Not yet ambushed at all (still invisible) - engage immediately regardless of
            // distance/cooldown, same as every subsequent teleport condition below.
            let first_engage = entity.metadata.is_invisible;
            if first_engage || state.teleport_cooldown.is_ready() || distance >= ambush.reengage_distance {
                teleport_behind(entity, target_pos, target_yaw);
                if first_engage {
                    entity.metadata.is_invisible = false;
                    world.send_metadata_update(entity_id);
                }
                state.teleport_cooldown.trigger(ambush.reengage_interval_ticks);

                // Fresh engagement, or a mid-fight reposition - either way the old weave/volley
                // phase no longer applies to the new spot. A brand new engagement also rerolls
                // which way it spins (held fixed for the rest of the fight, per
                // `MobAiState::orbit_clockwise`'s own doc comment); a mid-fight reposition keeps
                // whatever direction it already had.
                if first_engage {
                    state.orbit_clockwise = rand::rng().random_bool(0.5);
                }
                if state.volley_arrows_left > 0 {
                    attack::lower_bow(world, entity_id, state.archetype);
                    attack::set_drawing_bow(entity, world, false);
                }
                state.volley_arrows_left = 0;
                state.weave_cooldown.trigger(WEAVE_CYCLE_TICKS);
            } else {
                state.teleport_cooldown.tick();
            }

            if state.volley_arrows_left > 0 {
                // Mid bow-volley - "completely stops moving" per explicit request, just keeps
                // facing the target and fires on a fixed cadence.
                movement::face_toward(entity, target_pos);
                state.volley_shot_cooldown.tick();
                if state.volley_shot_cooldown.is_ready() {
                    fire_volley_arrow(world, entity, target_pos);
                    state.volley_arrows_left -= 1;
                    if state.volley_arrows_left > 0 {
                        state.volley_shot_cooldown.trigger(VOLLEY_SHOT_INTERVAL_TICKS);
                    } else {
                        attack::lower_bow(world, entity_id, state.archetype);
                        attack::set_drawing_bow(entity, world, false);
                        state.weave_cooldown.trigger(WEAVE_CYCLE_TICKS);
                    }
                }
            } else {
                // Continuously orbits *and* attacks - per explicit follow-up correction ("way
                // more offensive, less dodging... hitting me almost constantly"): an earlier
                // version only actually swung at the peak of a discrete weave-in dart, then
                // spent an equal stretch weaving back out (retreating right after landing a
                // hit), which read as dodging rather than relentless pressure. `ORBIT_RADIUS`
                // already sits inside `AttackModule::Melee`'s own range, so simply attacking
                // every tick while orbiting (gated by the attack module's own short cooldown -
                // see `profile.rs`'s Shadow Assassin entry) means it's swinging on essentially
                // every cooldown window instead of only once per full weave cycle. Radially
                // corrects back toward `ORBIT_RADIUS` (positive bias = outward) rather than a
                // flat 0.0, so it actually settles into and holds a consistent ring instead of
                // drifting wherever tangential movement alone leaves it. Recomputed fresh here
                // (not the `distance` above, which is stale on a tick that just teleported -
                // `entity.position` may have moved since).
                let current_distance = entity.position.distance_to(&target_pos);
                let radial_bias = ((ORBIT_RADIUS - current_distance) / ORBIT_RADIUS).clamp(-1.0, 1.0);
                movement::orbit_and_weave(entity, world, width, height, target_pos, state.orbit_clockwise, radial_bias, combat_speed_bps * ORBIT_SPEED_MULTIPLIER);
                movement::face_toward(entity, target_pos);

                state.attack_cooldowns.primary.tick();
                state.attack_cooldowns.secondary.tick();
                attack::try_attack(world, entity, &mut state, profile.attack, target_pos);

                // Still rolls a bow volley on the same cycle timer as before - a real pause, but
                // an intentional/occasional one (see `VOLLEY_CHANCE`), not the constant
                // dart-in-dart-out this replaced.
                state.weave_cooldown.tick();
                if state.weave_cooldown.is_ready() {
                    state.weave_cooldown.trigger(WEAVE_CYCLE_TICKS);
                    if rand::rng().random_bool(VOLLEY_CHANCE) {
                        attack::raise_bow(world, entity_id, state.archetype);
                        attack::set_drawing_bow(entity, world, true);
                        state.volley_arrows_left = VOLLEY_ARROW_COUNT;
                        state.volley_shot_cooldown = Cooldown::ready();
                    }
                }
            }

            world.entity_mob_ai.insert(entity_id, state);
            return;
        }

        if state.bow_draw_ticks > 0 {
            // Mid bow-draw/shot - plants and aims instead of also trying to close the distance
            // at the same time, per explicit report ("when they shoot bows, they should stop
            // moving"). `bow_draw_ticks` is only ever nonzero for a `uses_bow` ranged archetype
            // (see `attack::try_ranged`), so this can't accidentally freeze a pure-melee mob.
            movement::face_toward(entity, target_pos);
        } else if state.first_sight_pending {
            movement::steer_toward(entity, world, width, height, target_pos, FIRST_SIGHT_SPEED_BPS * profile.speed_multiplier, false);
            state.first_sight_pending = false;
        } else {
            let (steer_pos, on_path) = pathfinding::next_steer_point(world, entity_id, mob_pos, target_pos, width, height);
            movement::apply_movement_style(entity, world, width, height, profile.movement, target_pos, steer_pos, combat_speed_bps, on_path);
        }

        // "Run and jump to the player until they get close" per explicit request - only while
        // genuinely still closing the distance from far away, not once holding a ranged stance
        // (about to shoot), mid-shot (see the `bow_draw_ticks` freeze above - also skipped here),
        // or already near enough to fight. Mirrors `MovementStyle`'s own steer-vs-hold condition
        // rather than reusing `in_strike_range` below, which alone wouldn't distinguish "holding
        // at range to shoot" from "still running in".
        if profile.hops_while_approaching && state.bow_draw_ticks == 0 {
            let still_closing = match profile.movement {
                MovementStyle::HybridMeleeRanged { ranged_preferred, ranged_tolerance, .. } => {
                    entity.position.distance_to(&target_pos) > ranged_preferred + ranged_tolerance
                }
                // Stops the moment it's close enough to start orbiting/striking - without this
                // it kept hopping even standing right on top of the player (explicit report:
                // "shouldn't be trying to jump inside me").
                MovementStyle::CircleStrafe { melee_range, .. } | MovementStyle::LightStrafe { melee_range, .. } => {
                    entity.position.distance_to(&target_pos) > melee_range
                }
                MovementStyle::Approach => true,
                _ => false,
            };
            if still_closing && entity.ticks_existed % HOP_INTERVAL_TICKS == 0 {
                physics::try_jump(entity);
            }
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
            attack::AttackModule::Melee { range, .. } => entity.position.distance_to(&target_pos) <= range,
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

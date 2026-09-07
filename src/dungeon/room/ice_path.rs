//! Ice Path puzzle - a silverfish sits on an ice floor; punching it slides it in a straight line
//! in whatever direction the puncher is currently facing (snapped to the nearest cardinal
//! direction - the room is built on a block grid, so a free-angle slide wouldn't cleanly line up
//! against any wall), until it reaches a solid block and stops. Researched, not guessed - see
//! memory `project-quiz-puzzle-research`'s sibling research for Ice Path (no dedicated memory
//! file yet) and the Hypixel wiki's Dungeon Puzzle Rooms page:
//!
//! - Floor 3+, failable, 0 secrets, reward is a lootable Blessing chest (unlike Quiz, which
//!   delivers its buff directly with no chest). Per explicit request, that chest's own opening -
//!   not the silverfish reaching the goal - is the actual win condition, same "reaching it just
//!   reveals the chest, opening it solves the puzzle" pattern Teleport Maze's reward chest uses.
//! - Neither Odin (the real client solver mod) nor RustClear (this project's usual donor repo)
//!   ever automated or ported this puzzle - Odin only lists "Ice Path" as a known room name with
//!   no solver logic at all, and only automates the unrelated "Ice Fill" puzzle (already ported
//!   here as `ice_fill.rs` - walking on tiles, not this). So unlike Quiz, there's no chat-protocol
//!   or timing ground truth to reverse-engineer against here - the win condition below (goal
//!   tiles, exploding gate wall, sound/particle) is an explicit product spec, not a research find.
//! - Real fail conditions per the wiki/forums: killing the silverfish, or (historically) blocking
//!   its path to redirect it - the latter was patched to be an explicit fail rather than exploit.
//!   Neither is wired up yet.
//!
//! Spawn position and slide speed are explicit product decisions, not researched values: the
//! silverfish spawns at room-relative (8,67,9) - y=67 sits directly on this room's captured ice
//! floor (a solid PackedIce plaza at y=66, see the room capture
//! `room_data/rooms/39,ice_path,-60,-276.json`) - and slides at 5 blocks per 10 ticks (0.5
//! blocks/tick).
//!
//! Movement is driven by `EntityImpl::tick`, called automatically every server tick for every
//! live entity (see `Entity::tick` in `server::entity::entity`) - no manual `Server::schedule`
//! chaining needed here, unlike Quiz's discrete button-press steps. Position updates are left to
//! `Entity::tick`'s own generic "position changed since last tick" broadcast (a plain absolute
//! teleport) rather than `ai::projectile`'s hand-rolled relative-move packets - that optimization
//! exists there for fast projectiles (multiple blocks/tick) where a teleport-every-tick reads as
//! choppy; at 0.5 blocks/tick or an even slower tick rate, a teleport every tick is already smooth.
//!
//! The floating TNT "hat" is an Armor Stand, not a real dropped-item entity - a `DroppedItem` was
//! tried for a closer visual match, but it consistently rendered a step ahead of the fish while
//! sliding (correct again the instant it stopped) no matter how its position/broadcast timing was
//! tuned - real vanilla clients interpolate different entity *classes* over different tick
//! windows, and a `DroppedItem`'s window apparently doesn't match a `Silverfish`'s, a client-side
//! difference no server-side fix can close. An Armor Stand is the same `Living` class family as a
//! Silverfish, so it tracks with zero lag - confirmed per explicit request in exchange for a less
//! exact "dropped item" look.

use crate::dungeon::room::room::Room;
use crate::dungeon::room::secrets::{DungeonSecret, SecretType};
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{EntityEquipment, Particles, SoundEffect};
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::net::var_int::VarInt;
use crate::server::block::block_collision::is_block_passable;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::entity::dungeon_mobs::ai::movement::yaw_towards;
use crate::server::entity::entity::{Entity, EntityId, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::{ClientId, Player};
use crate::server::utils::direction::Direction;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;
use std::cell::RefCell;
use std::rc::Rc;

/// 5 blocks per 10 ticks, per explicit request.
const BLOCKS_PER_TICK: f64 = 5.0 / 10.0;

/// Room-relative spawn position, per explicit request - y=67 is feet-height standing directly on
/// this room's captured ice floor (a solid PackedIce plaza at y=66).
const SPAWN_POS: BlockPos = BlockPos { x: 8, y: 67, z: 9 };

/// Room-relative floor tiles that solve the puzzle the instant the silverfish touches any of
/// them, per explicit request - y=66 is the floor block itself (the fish's feet sit at y=67,
/// one above), matching `SPAWN_POS`'s same floor.
const GOAL_TILES: [BlockPos; 3] = [
    BlockPos { x: 16, y: 66, z: 24 },
    BlockPos { x: 15, y: 66, z: 24 },
    BlockPos { x: 14, y: 66, z: 24 },
];

/// Room-relative corners (inclusive) of the gate wall that explodes away on solve, per explicit
/// request.
const GATE_WALL_MIN: BlockPos = BlockPos { x: 14, y: 68, z: 25 };
const GATE_WALL_MAX: BlockPos = BlockPos { x: 16, y: 71, z: 25 };

/// How fast the silverfish visually turns to face a new travel direction, in degrees/tick - a
/// made-up default (no real capture to measure a turn speed against, unlike the slide speed
/// itself which was given directly), picked to read as a deliberate turn rather than an instant
/// snap while still finishing well within a typical slide's length.
const TURN_DEGREES_PER_TICK: f32 = 30.0;

/// The Armor Stand's own (feet-level) Y position, relative to the silverfish's feet - NOT the
/// visible height of the equipped TNT itself. Vanilla renders a mob's helmet-slot item at that
/// model's own head-bone height above ITS feet, not at the stand's raw entity position - a first
/// attempt at `0.4` (a small stand's feet 0.4 above the fish's) put the equipped TNT at roughly
/// player chest height, meaning the small stand's own head-bone sits ~1.0 blocks above its own
/// feet. `-0.65` (that same math run in reverse, targeting ~0.35 visible height above the fish)
/// was confirmed positioned correctly but too low - raised per explicit request.
const TNT_Y_OFFSET: f64 = -0.5;

/// Room-relative position of the reward chest sitting behind the gate wall - per explicit
/// request, whatever chest already exists there in the captured room data gets replaced with a
/// real "blessing" chest (reuses the same `DungeonSecret`/`blessing_texture` mechanism Tic Tac
/// Toe's and Teleport Maze's own reward chests use).
const REWARD_CHEST_POS: BlockPos = BlockPos { x: 15, y: 67, z: 28 };

/// Same shared "blessing" skull skin Tic Tac Toe's/Teleport Maze's own reward chests use - see
/// `EssenceEntityImpl`'s doc comment (`secrets.rs`) for what setting `DungeonSecret::blessing_texture`
/// actually triggers on open (the ascending `note.harp` sequence + floating, spinning skull).
const REWARD_BLESSING_TEXTURE: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZTkzZTIwNjg2MTc4NzJjNTQyZWNkYTFkMjdkZjRlY2U5MWM2OTk5MDdiZjMyN2M0ZGRiODUzMDk0MTJkMzkzOSJ9fX0=";

/// Sets up the Ice Path puzzle for `room` if it actually is one - spawns the silverfish at its
/// fixed room-relative position, and precomputes the goal tiles' and gate wall's world-space
/// positions (rotation-correct, same per-block conversion `quiz.rs`'s button setup uses, rather
/// than transforming just the box corners - a 90°-rotated axis-aligned box doesn't stay axis-
/// aligned in the same way if you naively rotate only its corners). No-op for every other room
/// (the caller already filters by name and by `Room::ice_path_spawned`). Called once from
/// `Dungeon::tick`'s room-entry hook, the first time a player actually crosses into the room -
/// same idiom `quiz::setup`/`three_weirdos::setup` already use.
pub fn setup(room: &Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Ice Path" {
        return;
    }

    let world_pos = room.get_world_block_pos(&SPAWN_POS);
    let position = DVec3::new(world_pos.x as f64 + 0.5, world_pos.y as f64, world_pos.z as f64 + 0.5);

    let goal_tiles = GOAL_TILES.map(|p| room.get_world_block_pos(&p));

    let mut gate_wall = Vec::new();
    for x in GATE_WALL_MIN.x..=GATE_WALL_MAX.x {
        for y in GATE_WALL_MIN.y..=GATE_WALL_MAX.y {
            for z in GATE_WALL_MIN.z..=GATE_WALL_MAX.z {
                gate_wall.push(room.get_world_block_pos(&BlockPos::new(x, y, z)));
            }
        }
    }

    // Spawned before the fish itself - an invisible, ai-disabled Armor Stand with the TNT
    // equipped in its helmet slot (see `TntHatImpl`), the same trick `PickupEntityImpl::spawn`
    // (`secrets.rs`) already uses for the floating key/Superboom TNT pickups elsewhere in this
    // codebase. See the module doc comment for why an Armor Stand, not a real `DroppedItem`.
    let hat_metadata = {
        let mut m = EntityMetadata::new(EntityVariant::ArmorStand);
        m.is_invisible = true;
        m.ai_disabled = true;
        m.is_small_armor_stand = true;
        m
    };
    let hat_pos = DVec3::new(position.x, position.y + TNT_Y_OFFSET, position.z);
    let tnt_entity_id = world.spawn_entity(hat_pos, hat_metadata, TntHatImpl).ok();

    let mut metadata = EntityMetadata::new(EntityVariant::Silverfish);
    metadata.ai_disabled = true;

    let chest_pos = room.get_world_block_pos(&REWARD_CHEST_POS);
    let state = SilverfishSlider {
        sliding: false,
        velocity_per_tick: DVec3::ZERO,
        target_yaw: 0.0,
        solved: false,
        goal_tiles,
        gate_wall,
        tnt_entity_id,
        room_index,
        chest_pos,
        rotation: room.rotation,
    };
    let _ = world.spawn_entity(position, metadata, state);

    // The chest block itself is placed right away, per explicit request - it's there behind the
    // gate wall from the start, just not yet a real, clickable chest (no `interactable_blocks`
    // entry registered for it yet). `trigger_win` is what actually registers it as openable, once
    // the silverfish blows the gate wall open - see `spawn_reward_chest`'s doc comment.
    let mut chest_block = Blocks::Chest { direction: Direction::North };
    chest_block.rotate(room.rotation);
    world.set_block_at(chest_block, chest_pos.x, chest_pos.y, chest_pos.z);
}

/// Invisible Armor Stand's own `EntityImpl` - just equips the TNT into its helmet slot on spawn
/// (Armor Stands, unlike Silverfish, render head-slot equipment - a Silverfish's own real vanilla
/// renderer doesn't draw held/worn equipment at all). No `tick()` behavior of its own -
/// `SilverfishSlider::tick` drives its position externally every tick to keep it following the
/// fish, since that's the entity that actually knows where the fish is.
struct TntHatImpl;

impl EntityImpl for TntHatImpl {
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        let item = ItemStack::new(46); // TNT
        packet_buffer.write_packet(&EntityEquipment {
            entity_id: VarInt(entity.id),
            item_slot: 4, // Helmet slot
            item_stack: Some(item.clone()),
        });

        let world = entity.world_mut();
        for player in world.players.values_mut() {
            player.write_packet(&EntityEquipment {
                entity_id: VarInt(entity.id),
                item_slot: 4,
                item_stack: Some(item.clone()),
            });
        }
    }

    fn tick(&mut self, _entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {}
}

/// Registers `chest_pos` (world-space, resolved once in `setup`) as a real, openable "blessing"
/// chest, facing the room's own canonical north rotated to match its actual placement (`rotation`,
/// captured once in `setup` - see `SilverfishSlider::rotation`'s doc comment for why it has to be
/// threaded through state instead of read live). The chest *block* itself is placed early, in `setup` - it
/// sits there behind the gate wall from the moment the room is entered - but per explicit request
/// it must not be *openable* until the silverfish actually blows the gate open, so only this part
/// (the `world.interactable_blocks` registration every block-click dispatch in this codebase goes
/// through - see `packet_handling.rs`) is deferred to `trigger_win`. Before that, right-clicking
/// it is a plain no-op: nothing is registered at that position yet.
///
/// `puzzle_room_index` is set (per explicit request: "the win condition is grabbing the chest") -
/// same "the chest's own opening is the actual win condition" pattern Teleport Maze's reward
/// chest uses (see `DungeonSecret::puzzle_room_index`'s doc comment): the generic `Chest` interact
/// handler in `block_interact_action.rs` is what marks the room solved and broadcasts the PUZZLE
/// SOLVED message the first time this chest opens - `trigger_win` itself only blows the gate open
/// and places the chest, it does neither of those itself.
fn spawn_reward_chest(chest_pos: BlockPos, room_index: usize, rotation: Direction, world: &mut World) {
    let secret_rc = Rc::new(RefCell::new(DungeonSecret::new(
        SecretType::Chest { direction: Direction::North.rotate(rotation) },
        chest_pos,
        0.0,
    )));
    {
        let mut secret = secret_rc.borrow_mut();
        secret.blessing_texture = Some(REWARD_BLESSING_TEXTURE);
        secret.puzzle_room_index = Some(room_index);
        // Ice Path is a real 0-secret room per the wiki - this chest is a bonus reward, not one
        // of the room's counted secrets.
        secret.counts_as_secret = false;
    }
    let secret_ref = secret_rc.borrow_mut();
    DungeonSecret::spawn_into_world(&secret_rc, secret_ref, world);
}

/// Per-entity state for the Ice Path silverfish. `sliding`/`velocity_per_tick` together track
/// whether (and how) it's currently moving - a punch while already sliding is ignored, matching
/// the real "commits to a full slide until it hits something" mechanic rather than letting a
/// second punch redirect it mid-flight.
struct SilverfishSlider {
    sliding: bool,
    velocity_per_tick: DVec3,
    /// Yaw the fish is turning toward - set the instant a new slide starts (punch or arrow), and
    /// approached gradually in `tick()` via `step_yaw_towards` rather than snapped to instantly.
    target_yaw: f32,
    /// Set once the silverfish has reached the goal, so a tick already in flight when that fires
    /// can't process twice (the entity is despawned right then, but its own `tick()` call that
    /// triggered it is still running at that point). NOT the same as the puzzle actually being
    /// solved - see `trigger_win`'s doc comment - just guards this entity's own one-shot cleanup.
    solved: bool,
    /// World-space floor tiles (see `GOAL_TILES`) - reaching any of these blows the gate open.
    goal_tiles: [BlockPos; 3],
    /// World-space blocks (see `GATE_WALL_MIN`/`MAX`) that explode away on reaching the goal.
    gate_wall: Vec<BlockPos>,
    /// The floating TNT "hat" armor stand's own entity id (see `TntHatImpl`) - `None` only if it
    /// somehow failed to spawn. Followed every tick (see `tick()`) and despawned alongside the
    /// fish in `trigger_win`.
    tnt_entity_id: Option<EntityId>,
    /// This room's index - needed only to pass through to `spawn_reward_chest`'s
    /// `puzzle_room_index` once `trigger_win` actually places the chest.
    room_index: usize,
    /// World-space position of the reward chest (see `REWARD_CHEST_POS`), resolved once in
    /// `setup` - not placed as a real chest until `trigger_win` calls `spawn_reward_chest`.
    chest_pos: BlockPos,
    /// The room's own actual placed rotation, captured once in `setup` - `spawn_reward_chest`
    /// needs it to face the reward chest the room's own canonical north, not the world compass
    /// direction, and `trigger_win` has no other way to reach the live `Room` from here.
    rotation: Direction,
}

impl SilverfishSlider {
    /// Commits the fish to a new slide in `direction` at the fixed puzzle speed, and points
    /// `target_yaw` the same way so `tick()` gradually turns to face it. Shared by `interact()`
    /// (a punch) and `on_projectile_hit()` (a Terminator arrow) - both redirect the fish "the
    /// same way", just deriving `direction` differently (the puncher's own facing vs. the
    /// arrow's actual flight direction).
    fn start_sliding(&mut self, direction: Direction) {
        let (dx, _, dz) = direction.get_offset();
        self.velocity_per_tick = DVec3::new(dx as f64 * BLOCKS_PER_TICK, 0.0, dz as f64 * BLOCKS_PER_TICK);
        self.target_yaw = yaw_towards(dx as f64, dz as f64);
        self.sliding = true;
    }
}

impl EntityImpl for SilverfishSlider {
    fn tick(&mut self, entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
        if self.solved {
            return;
        }

        if self.sliding {
            entity.yaw = step_yaw_towards(entity.yaw, self.target_yaw, TURN_DEGREES_PER_TICK);

            let next = entity.position + self.velocity_per_tick;
            let feet_y = entity.position.y.floor() as i32;
            let floor_pos = BlockPos::new(next.x.floor() as i32, feet_y - 1, next.z.floor() as i32);

            if self.goal_tiles.contains(&floor_pos) {
                entity.position = next;
                trigger_win(self, entity);
                return;
            }

            let target = BlockPos::new(next.x.floor() as i32, feet_y, next.z.floor() as i32);
            let world = entity.world_mut();
            if is_block_passable(world.get_block_at(target.x, target.y, target.z)) {
                entity.position = next;
            } else {
                // Hit a wall - stop and snap to the center of the block it was still standing
                // in, so it doesn't end up clipped halfway into the wall.
                self.sliding = false;
                self.velocity_per_tick = DVec3::ZERO;
                let stopped = BlockPos::new(entity.position.x.floor() as i32, feet_y, entity.position.z.floor() as i32);
                entity.position = DVec3::new(stopped.x as f64 + 0.5, entity.position.y, stopped.z as f64 + 0.5);
            }
        }

        // Keeps the floating TNT positioned above the fish - every tick regardless of sliding,
        // per explicit request ("constantly above it"). Deliberately placed AFTER all of the
        // fish's own movement/wall-stop-snap above, so it always reflects this tick's final
        // position, not a stale pre-move one. A plain position set is enough here - an Armor
        // Stand is the same `Living` entity class as the Silverfish, so both interpolate
        // identically client-side and stay in perfect sync (see the module doc comment for why a
        // real `DroppedItem` entity, tried first, couldn't do this regardless of how its
        // position/broadcast timing was tuned).
        if let Some(tnt_id) = self.tnt_entity_id {
            let hat_pos = DVec3::new(entity.position.x, entity.position.y + TNT_Y_OFFSET, entity.position.z);
            let world = entity.world_mut();
            if let Some((tnt_entity, _)) = world.entities.get_mut(&tnt_id) {
                tnt_entity.position = hat_pos;
            }
        }
    }

    fn interact(&mut self, _entity: &mut Entity, player: &mut Player, action: &EntityInteractionType) -> bool {
        if self.solved || *action != EntityInteractionType::Attack || self.sliding {
            return false;
        }

        self.start_sliding(cardinal_from_yaw(player.yaw));
        true
    }

    /// A Terminator arrow hitting the fish redirects it "the same way" a punch does, per explicit
    /// request - direction comes from the arrow's own flight velocity (snapped to the nearest
    /// cardinal axis), not the shooter's current look angle, since a real (gravity-arced) shot's
    /// actual path can differ from wherever the shooter happens to be aiming by the time it lands.
    fn on_projectile_hit(&mut self, _entity: &mut Entity, velocity: DVec3, _shooter_id: ClientId) -> bool {
        if self.solved || self.sliding {
            return true;
        }

        self.start_sliding(cardinal_from_horizontal(velocity.x, velocity.z));
        true
    }

    /// Explicit opt-in - see `EntityImpl::wants_projectile_hits`'s doc comment for why this can't
    /// just be inferred from visibility/entity-type the way an earlier attempt tried.
    fn wants_projectile_hits(&self) -> bool {
        true
    }
}

/// Runs the instant the silverfish's slide carries it onto one of `GOAL_TILES` - blows open the
/// gate wall (particle + sound) and despawns the fish (it "explodes"). Does NOT mark the puzzle
/// solved - per explicit request, the actual win condition is opening the reward chest this
/// reveals (see `spawn_reward_chest`'s `puzzle_room_index`), not reaching the goal itself.
fn trigger_win(state: &mut SilverfishSlider, entity: &mut Entity) {
    state.solved = true;
    state.sliding = false;

    let pos = entity.position;
    let world = entity.world_mut();
    world.despawn_entity(entity.id);
    if let Some(tnt_id) = state.tnt_entity_id {
        world.despawn_entity(tnt_id);
    }

    for &block_pos in &state.gate_wall {
        world.set_block_at(Blocks::Air, block_pos.x, block_pos.y, block_pos.z);
    }

    let particles = Particles {
        particle_id: 1, // largeexplode
        long_distance: true,
        x: pos.x as f32,
        y: pos.y as f32,
        z: pos.z as f32,
        offset_x: 0.0,
        offset_y: 0.0,
        offset_z: 0.0,
        speed: 0.0,
        count: 0,
    };
    let sound = SoundEffect {
        sound: Sounds::RandomExplode.id(), // 1.8.9 equivalent of modern `entity.generic.explode`
        pos_x: pos.x,
        pos_y: pos.y,
        pos_z: pos.z,
        volume: 1.0,
        pitch: 1.286,
    };
    for player in world.players.values_mut() {
        player.write_packet(&particles);
        player.write_packet(&sound);
    }

    // The chest block has been sitting there since `setup` - only now does it actually become
    // openable, see `spawn_reward_chest`'s doc comment.
    spawn_reward_chest(state.chest_pos, state.room_index, state.rotation, world);
}

/// Snaps a yaw to the nearest of the 4 cardinal directions, using the same convention already
/// verified elsewhere for reading a *player's* own facing (`items::ender_pearl`'s forward-vector
/// formula, and `ai::movement::yaw_towards`'s doc comment): 0=South/+z, 90=West/-x, 180=North/-z,
/// 270=East/+x. Deliberately NOT `ai::projectile::face_velocity`'s `atan2(x, z)` formula, which
/// uses the opposite East/West sign - that one orients a projectile along its own velocity, a
/// different (and explicitly non-interchangeable, per that function's own doc comment) convention
/// from reading a player's facing. Using it here was the actual bug behind "sometimes hits the
/// silverfish the opposite direction" - it silently swapped every East/West punch (North/South
/// happened to land on the same buckets either way, so only sideways punches were ever wrong).
fn cardinal_from_yaw(yaw: f32) -> Direction {
    let normalized = ((yaw % 360.0) + 360.0) % 360.0;
    match (((normalized + 45.0) / 90.0) as i32) % 4 {
        0 => Direction::South,
        1 => Direction::West,
        2 => Direction::North,
        _ => Direction::East,
    }
}

/// Snaps a horizontal velocity vector to whichever cardinal axis it's more aligned with - the
/// dominant-axis test sidesteps the whole yaw-sign-convention question `cardinal_from_yaw` has to
/// deal with (an arrow's velocity is already a real XZ vector, not an angle to decode), so this
/// can't repeat that bug class even if the two ever disagreed.
fn cardinal_from_horizontal(dx: f64, dz: f64) -> Direction {
    if dx.abs() > dz.abs() {
        if dx > 0.0 { Direction::East } else { Direction::West }
    } else if dz > 0.0 { Direction::South } else { Direction::North }
}

/// Turns `current` toward `target` by at most `max_delta` degrees, taking the shorter way around
/// (e.g. 350°->10° steps by +20, not -340) - a gradual turn instead of an instant snap, per
/// explicit request. No-op once `current` already equals `target`.
fn step_yaw_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    let mut diff = (target - current) % 360.0;
    if diff > 180.0 { diff -= 360.0; }
    if diff < -180.0 { diff += 360.0; }
    let step = diff.clamp(-max_delta, max_delta);
    let mut result = (current + step) % 360.0;
    if result < 0.0 { result += 360.0; }
    result
}

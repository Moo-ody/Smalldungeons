//! Creeper Beams puzzle, per the exact behavior spec given for this session (not re-derived
//! from external research - the mechanics/state-machine below follow it literally):
//!
//! A Creeper floats at the room's center with cloud particles drifting beneath it. The player
//! pairs sea lanterns by hitting them (in practice, with a Terminator arrow - see
//! `ai::projectile::MobProjectileImpl::on_block_hit`, which is what actually routes an arrow
//! hit into `interact_lantern` below, the same as a direct click would). First hit selects a
//! lantern; a second hit pairs it with the first, turning both into prismarine and connecting
//! them with a guardian-style beam (two stationary Guardians mutually targeting each other -
//! vanilla's laser only ever draws from a live guardian's own eye toward whatever it's
//! targeting, so there's no packet for "a beam between two arbitrary fixed points" - see
//! `EntityVariant::Guardian`'s doc comment). A pair is *correct* if that beam's line intersects
//! the creeper; correct pairs lock permanently and count toward the 4 needed to solve it
//! (charging the creeper at 3). Incorrect pairs stay resettable - hitting either of their
//! blocks reverts both back to inactive sea lanterns and removes the beam, without touching any
//! other pair.
//!
//! Lantern positions and the creeper's own position were both recovered directly from this
//! repo's one recorded room layout (`51,creeper_beams,-60,-528.json`'s `block_data`, decoded for
//! every Sea Lantern block) rather than guessed - the creeper's position specifically was
//! reconstructed as the least-squares closest point to several known real lantern-pair lines
//! (crowd-sourced candidate pairs cross-checked against which of those 22 real lanterns actually
//! exist in this layout), landing tightly within half a block of (15, 77, 15) across every line
//! checked.

use crate::dungeon::room::room::Room;
use crate::net::protocol::play::clientbound::{BlockAction, Particles, SoundEffect};
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityId, NoEntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::particles::ParticleTypes;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Room-relative (pre-rotation) position of the Creeper - see the module doc comment for how
/// this was recovered. Not a block position (it floats), but anchored to one for the
/// room-rotation transform (`Room::get_world_block_pos` only operates on integer positions).
const CREEPER_ANCHOR: BlockPos = BlockPos { x: 15, y: 77, z: 15 };

/// All 22 real Sea Lantern positions in this room's one recorded layout, decoded directly from
/// its `block_data` (room-relative, pre-rotation - `Room::get_world_block_pos` handles rotation
/// the same way every other puzzle's fixed positions in this codebase already do).
const LANTERN_POSITIONS: [BlockPos; 22] = [
    BlockPos { x: 23, y: 66, z: 5 },
    BlockPos { x: 15, y: 66, z: 26 },
    BlockPos { x: 15, y: 67, z: 15 },
    BlockPos { x: 10, y: 69, z: 27 },
    BlockPos { x: 6, y: 70, z: 5 },
    BlockPos { x: 21, y: 70, z: 27 },
    BlockPos { x: 18, y: 71, z: 2 },
    BlockPos { x: 2, y: 71, z: 13 },
    BlockPos { x: 4, y: 72, z: 8 },
    BlockPos { x: 25, y: 72, z: 23 },
    BlockPos { x: 24, y: 74, z: 6 },
    BlockPos { x: 15, y: 74, z: 15 },
    BlockPos { x: 2, y: 75, z: 16 },
    BlockPos { x: 3, y: 76, z: 18 },
    BlockPos { x: 25, y: 76, z: 23 },
    BlockPos { x: 5, y: 76, z: 24 },
    BlockPos { x: 24, y: 77, z: 7 },
    BlockPos { x: 26, y: 78, z: 12 },
    BlockPos { x: 27, y: 78, z: 14 },
    BlockPos { x: 18, y: 82, z: 8 },
    BlockPos { x: 9, y: 83, z: 17 },
    BlockPos { x: 15, y: 84, z: 13 },
];

/// Room-relative reveal position for the reward chest (confirmed: real Hypixel spawns it here on
/// solve, watched for directly by community solvers).
const CHEST_REVEAL_POS: BlockPos = BlockPos { x: 15, y: 69, z: 15 };

/// Perpendicular distance (blocks) from the creeper within which a beam counts as "intersecting"
/// it. Every real correct-pair line recovered for this layout passed within ~0.4 blocks of the
/// fitted creeper position, so this is generous on purpose - a beam that's genuinely aimed
/// through the creeper shouldn't miss this over floating-point/parametrization slop, while still
/// being far tighter than the room's own scale (lanterns are 10-30+ blocks apart).
const BEAM_HIT_TOLERANCE: f64 = 1.5;

const CORRECT_PAIRS_TO_CHARGE: u32 = 3;
const CORRECT_PAIRS_TO_SOLVE: u32 = 4;

/// Ticks between each puff of cloud particles under the floating creeper.
const CLOUD_INTERVAL_TICKS: u32 = 10;

#[derive(Debug, Clone, Copy)]
enum LanternStatus {
    /// Block state 1: inactive sea lantern, available to be selected.
    Available,
    /// Block state 2: selected sea lantern, waiting for a second lantern to pair with.
    Selected,
    /// Block states 3/4 (distinguished by `correct`): paired prismarine, connected to `partner`
    /// by the guardian beam formed from `guardians`. Incorrect pairs stay resettable; correct
    /// ones lock permanently (see `interact_lantern`).
    Paired { partner: BlockPos, correct: bool, guardians: (EntityId, EntityId) },
}

#[derive(Debug)]
pub struct CreeperBeamsState {
    room_index: usize,
    creeper_world_pos: DVec3,
    creeper_entity_id: Option<EntityId>,
    lanterns: HashMap<BlockPos, LanternStatus>,
    correct_pairs: u32,
    resolved: bool,
    /// World tick each lantern position was last actually acted on - see `HIT_DEBOUNCE_TICKS`.
    last_hit_tick: HashMap<BlockPos, u64>,
}

/// Terminator fires 3 arrows per shot in a spread (see `items::terminator`) - at anything but
/// point-blank range it's common for 2 of them to land on the very same lantern a moment apart,
/// each independently triggering `interact_lantern`. Without debouncing, a *second* redundant
/// hit on a lantern that the *first* hit just paired would see it already `Paired` and (for an
/// incorrect pair) immediately reset it again - undoing the pair a tick after it formed, before
/// it's ever visible. This window only needs to cover "two arrows from one volley," not two
/// genuinely separate player actions, so it's kept short.
const HIT_DEBOUNCE_TICKS: u64 = 5;

/// Sets up the Creeper Beams puzzle for `room` if it actually is one - spawns the floating
/// Creeper and registers the 22 lanterns as interactable. No-op for every other room. Called
/// once from `Room::load_into_world`, same timing/pattern as `three_weirdos::setup`.
pub fn setup(room: &Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Creeper Beams" {
        return;
    }

    let creeper_anchor_world = room.get_world_block_pos(&CREEPER_ANCHOR);
    let creeper_world_pos = DVec3::new(
        creeper_anchor_world.x as f64 + 0.5,
        creeper_anchor_world.y as f64,
        creeper_anchor_world.z as f64 + 0.5,
    );

    let lantern_world_positions: [BlockPos; 22] = std::array::from_fn(|i| room.get_world_block_pos(&LANTERN_POSITIONS[i]));

    let mut lanterns = HashMap::new();
    for pos in lantern_world_positions {
        lanterns.insert(pos, LanternStatus::Available);
    }

    let state = Rc::new(RefCell::new(CreeperBeamsState {
        room_index,
        creeper_world_pos,
        creeper_entity_id: None,
        lanterns,
        correct_pairs: 0,
        resolved: false,
        last_hit_tick: HashMap::new(),
    }));

    let mut metadata = EntityMetadata::new(EntityVariant::Creeper { powered: false });
    metadata.ai_disabled = true;
    if let Ok(entity_id) = world.spawn_entity(creeper_world_pos, metadata, CreeperImpl { state: state.clone() }) {
        state.borrow_mut().creeper_entity_id = Some(entity_id);
    }

    for pos in lantern_world_positions {
        world.interactable_blocks.insert(pos, BlockInteractAction::CreeperBeamsLantern { state: state.clone() });
    }
}

/// The floating Creeper's own entity - motionless (no dungeon-mob AI, no physics), just a
/// periodic puff of cloud particles beneath it.
struct CreeperImpl {
    #[allow(dead_code)] // kept for symmetry/future use - the puzzle state itself already tracks the entity id separately
    state: Rc<RefCell<CreeperBeamsState>>,
}

impl crate::server::entity::entity::EntityImpl for CreeperImpl {
    fn tick(&mut self, entity: &mut Entity, _buffer: &mut crate::net::packets::packet_buffer::PacketBuffer) {
        if entity.ticks_existed % CLOUD_INTERVAL_TICKS != 0 {
            return;
        }
        let pos = entity.position;
        let particles = Particles {
            particle_id: ParticleTypes::Cloud.get_id(),
            long_distance: true,
            x: pos.x as f32,
            y: pos.y as f32 - 1.0,
            z: pos.z as f32,
            offset_x: 0.3,
            offset_y: 0.05,
            offset_z: 0.3,
            speed: 0.02,
            count: 3,
        };
        for player in entity.world_mut().players.values_mut() {
            player.write_packet(&particles);
        }
    }
}

/// `BlockInteractAction::CreeperBeamsLantern`'s handler - a player (directly, or via a Terminator
/// arrow hit - see the module doc comment) hit the lantern/prismarine block at `block_pos`.
pub fn interact_lantern(player: &mut Player, block_pos: &BlockPos, state: &Rc<RefCell<CreeperBeamsState>>) {
    let mut data = state.borrow_mut();
    if data.resolved {
        return;
    }

    let Some(&status) = data.lanterns.get(block_pos) else { return };

    // Debounce - see `HIT_DEBOUNCE_TICKS`'s doc comment (a second Terminator arrow from the same
    // volley landing on a lantern the first one just acted on).
    let current_tick = player.world_mut().tick_count;
    if let Some(&last_tick) = data.last_hit_tick.get(block_pos) {
        if current_tick.saturating_sub(last_tick) < HIT_DEBOUNCE_TICKS {
            return;
        }
    }
    data.last_hit_tick.insert(*block_pos, current_tick);

    match status {
        // Locked (correct) or already-waiting (this exact lantern is the current selection) -
        // both do nothing.
        LanternStatus::Paired { correct: true, .. } | LanternStatus::Selected => return,

        // Incorrect pair - reset both blocks back to inactive lanterns, remove the beam.
        LanternStatus::Paired { partner, correct: false, guardians } => {
            reset_pair(player, &mut data, *block_pos, partner, guardians);
        }

        LanternStatus::Available => {
            play_hit_sound(player);

            let already_selected = data.lanterns.iter()
                .find_map(|(&pos, &st)| matches!(st, LanternStatus::Selected).then_some(pos));

            match already_selected {
                None => {
                    data.lanterns.insert(*block_pos, LanternStatus::Selected);
                }
                Some(first) => {
                    form_pair(player, &mut data, first, *block_pos);
                }
            }
        }
    }
}

/// "Play the Minecraft XP pickup sound at a lowered pitch whenever a lantern is successfully
/// hit" - covers both selecting the first lantern and completing a pair with the second.
/// From the player, not the lantern - confirmed in-game (this is the real Minecraft XP-pickup
/// sound, which always plays as though it's coming from whoever picked it up, not from wherever
/// the XP orb was).
fn play_hit_sound(player: &mut Player) {
    let pos = player.position;
    player.write_packet(&SoundEffect {
        sound: Sounds::Orb.id(),
        pos_x: pos.x,
        pos_y: pos.y,
        pos_z: pos.z,
        volume: 1.0,
        pitch: 0.7,
    });
}

fn form_pair(player: &mut Player, data: &mut CreeperBeamsState, a: BlockPos, b: BlockPos) {
    play_hit_sound(player);

    // Also debounce `a` (not just `b`, already stamped by the caller) - covers a stray arrow
    // from the same volley landing on the *other* lantern of the pair that just formed instead
    // of the one that was actually aimed at.
    let current_tick = player.world_mut().tick_count;
    data.last_hit_tick.insert(a, current_tick);

    let correct = beam_intersects_creeper(a, b, data.creeper_world_pos);
    let creeper_entity_id = data.creeper_entity_id;

    let world = player.world_mut();
    world.set_block_at(Blocks::Prismarine { variant: 0 }, a.x, a.y, a.z);
    world.set_block_at(Blocks::Prismarine { variant: 0 }, b.x, b.y, b.z);
    let guardians = spawn_guardian_pair(world, a, b);

    data.lanterns.insert(a, LanternStatus::Paired { partner: b, correct, guardians });
    data.lanterns.insert(b, LanternStatus::Paired { partner: a, correct, guardians });

    if !correct {
        return;
    }

    data.correct_pairs += 1;

    if data.correct_pairs == CORRECT_PAIRS_TO_CHARGE {
        if let Some(creeper_id) = creeper_entity_id {
            if let Some((entity, _)) = world.entities.get_mut(&creeper_id) {
                entity.metadata.variant = EntityVariant::Creeper { powered: true };
            }
            world.send_metadata_update(creeper_id);
        }
    }

    if data.correct_pairs >= CORRECT_PAIRS_TO_SOLVE {
        complete_puzzle(player, data);
    }
}

/// One Guardian at `a` targeting an invisible Squid marker at `b` - the real, confirmed-working
/// technique for a "beam between two fixed points" (see `EntityVariant::Guardian`'s doc
/// comment). Both are spawned invisible - vanilla still renders an invisible guardian's laser,
/// just not its body, which is exactly the clean beam-only look wanted here. The squid is
/// spawned first so its id is already known when the guardian's own metadata is built.
///
/// Deliberately *not* also setting `ai_disabled` here (unlike most of this codebase's other
/// utility entities) - re-fetching `GuardianBeamAPI`'s real `PacketFactory.java` source to
/// re-verify why the beam wasn't showing turned up the actual real packet it sends:
/// `watcher.setObject(0, (byte) 32)` for both entities - the invisible bit *alone*. This
/// codebase's own `ai_disabled` flag also contributes an extra bit (0x40) to that same universal
/// status byte, on top of the invisible one - a bit with no real vanilla meaning at all (`NoAI`
/// is an NBT spawn tag in real Minecraft, never part of the DataWatcher/metadata protocol), so
/// sending it made this entity's status byte diverge from the exact one confirmed to work.
fn spawn_guardian_pair(world: &mut World, a: BlockPos, b: BlockPos) -> (EntityId, EntityId) {
    let pos_a = DVec3::new(a.x as f64 + 0.5, a.y as f64 + 0.5, a.z as f64 + 0.5);
    let pos_b = DVec3::new(b.x as f64 + 0.5, b.y as f64 + 0.5, b.z as f64 + 0.5);

    let mut squid_metadata = EntityMetadata::new(EntityVariant::Squid);
    squid_metadata.is_invisible = true;
    let squid_id = world.spawn_entity(pos_b, squid_metadata, NoEntityImpl).unwrap_or(-1);

    let mut guardian_metadata = EntityMetadata::new(EntityVariant::Guardian { target_entity_id: squid_id });
    guardian_metadata.is_invisible = true;
    let guardian_id = world.spawn_entity(pos_a, guardian_metadata, NoEntityImpl).unwrap_or(-1);

    (guardian_id, squid_id)
}

fn reset_pair(player: &mut Player, data: &mut CreeperBeamsState, a: BlockPos, b: BlockPos, guardians: (EntityId, EntityId)) {
    let world = player.world_mut();
    world.despawn_entity(guardians.0);
    world.despawn_entity(guardians.1);
    world.set_block_at(Blocks::SeaLantern, a.x, a.y, a.z);
    world.set_block_at(Blocks::SeaLantern, b.x, b.y, b.z);

    data.lanterns.insert(a, LanternStatus::Available);
    data.lanterns.insert(b, LanternStatus::Available);
}

/// A pair's beam "intersects the creeper" if the creeper's position is within
/// `BEAM_HIT_TOLERANCE` of the line *segment* (not infinite line) between the two paired blocks.
fn beam_intersects_creeper(a: BlockPos, b: BlockPos, creeper: DVec3) -> bool {
    let pa = DVec3::new(a.x as f64 + 0.5, a.y as f64 + 0.5, a.z as f64 + 0.5);
    let pb = DVec3::new(b.x as f64 + 0.5, b.y as f64 + 0.5, b.z as f64 + 0.5);
    let ab = pb - pa;
    let len_sq = ab.x * ab.x + ab.y * ab.y + ab.z * ab.z;
    if len_sq < 1e-9 {
        return pa.distance_to(&creeper) < BEAM_HIT_TOLERANCE;
    }

    let ac = creeper - pa;
    let t = ((ac.x * ab.x + ac.y * ab.y + ac.z * ab.z) / len_sq).clamp(0.0, 1.0);
    let closest = DVec3::new(pa.x + ab.x * t, pa.y + ab.y * t, pa.z + ab.z * t);
    closest.distance_to(&creeper) < BEAM_HIT_TOLERANCE
}

fn complete_puzzle(player: &mut Player, data: &mut CreeperBeamsState) {
    data.resolved = true;
    let room_index = data.room_index;
    let creeper_pos = data.creeper_world_pos;

    let particles = Particles {
        particle_id: ParticleTypes::ExplosionLarge.get_id(),
        long_distance: true,
        x: creeper_pos.x as f32,
        y: creeper_pos.y as f32,
        z: creeper_pos.z as f32,
        offset_x: 0.0,
        offset_y: 0.0,
        offset_z: 0.0,
        speed: 0.0,
        count: 0,
    };

    let dungeon = &mut player.server_mut().dungeon;
    let chest_world_pos = dungeon.rooms.get(room_index)
        .map(|room| room.get_world_block_pos(&CHEST_REVEAL_POS));

    let world = &mut player.server_mut().world;
    if let Some(creeper_id) = data.creeper_entity_id {
        world.despawn_entity(creeper_id);
    }

    if let Some(chest_pos) = chest_world_pos {
        world.set_block_at(Blocks::Chest { direction: crate::server::utils::direction::Direction::North }, chest_pos.x, chest_pos.y, chest_pos.z);
    }

    let message = format!("§a§lPUZZLE SOLVED! §7{} §esolved the Creeper Beams puzzle!", player.profile.username);
    for other in world.players.values_mut() {
        other.write_packet(&particles);
        other.write_packet(&SoundEffect {
            sound: Sounds::RandomExplode.id(),
            pos_x: creeper_pos.x,
            pos_y: creeper_pos.y,
            pos_z: creeper_pos.z,
            volume: 1.0,
            pitch: 1.0,
        });
        other.send_message(&message);
        if let Some(chest_pos) = chest_world_pos {
            other.write_packet(&BlockAction {
                block_pos: chest_pos,
                event_id: 1,
                event_data: 1,
                block_id: 54,
            });
        }
    }

    if let Some(room) = player.server_mut().dungeon.rooms.get_mut(room_index) {
        room.puzzle_completed = true;
    }
    player.server_mut().dungeon.update_map_for_room(room_index);
}

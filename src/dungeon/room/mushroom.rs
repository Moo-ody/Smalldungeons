use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::server::block::block_position::BlockPos;
use crate::server::utils::direction::Direction;

use super::crypts::{CryptBlock, rotate_block_pos};

#[derive(Debug, Deserialize)]
struct MushroomFile {
    schema: Option<u32>,
    rooms: std::collections::HashMap<String, MushroomRoomEntry>,
}

#[derive(Debug, Deserialize)]
struct MushroomRoomEntry {
    #[serde(default)]
    crypts: Vec<Vec<CryptBlock>>, // reuse same block format
}

static MUSHROOM_DATA: Lazy<Option<MushroomFile>> = Lazy::new(|| {
    let data = include_str!("../../room_data/misc/Mushroomdata.json");
    serde_json::from_str::<MushroomFile>(data).ok()
});

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// For a single crypt pattern, split mushrooms into (bottom, top) by y-extremes among small mushrooms (ids 39/40)
fn split_bottom_top(blocks: &[CryptBlock]) -> (Vec<CryptBlock>, Vec<CryptBlock>) {
    let mush: Vec<&CryptBlock> = blocks
        .iter()
        .filter(|b| matches!(b.block_id, Some(39 | 40)))
        .collect();
    if mush.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let min_y = mush.iter().map(|b| b.y).min().unwrap();
    let max_y = mush.iter().map(|b| b.y).max().unwrap();
    let bottom: Vec<CryptBlock> = mush
        .iter()
        .filter(|b| b.y == min_y)
        .map(|b| (*b).clone())
        .collect();
    let top: Vec<CryptBlock> = mush
        .iter()
        .filter(|b| b.y == max_y)
        .map(|b| (*b).clone())
        .collect();
    (bottom, top)
}

#[derive(Debug, Clone)]
pub struct MushroomSets {
    pub bottom: Vec<BlockPos>,
    pub top: Vec<BlockPos>,
    pub up: Vec<BlockPos>,
}

/// Load mushroom secret sets for a given room name; returns world-relative positions once rotated.
pub fn get_room_mushrooms(room_name: &str, rotation: Direction, corner_pos: &BlockPos) -> Vec<MushroomSets> {
    let Some(file) = MUSHROOM_DATA.as_ref() else { return Vec::new(); };

    let want = normalize(room_name);
    // Find entry either by exact key or normalized
    let entry = if let Some(e) = file.rooms.get(room_name) {
        Some(e)
    } else {
        file.rooms
            .iter()
            .find(|(k, _)| normalize(k) == want)
            .map(|(_, v)| v)
    };

    let Some(entry) = entry else { return Vec::new(); };

    let mut out: Vec<MushroomSets> = Vec::new();
    let patterns = &entry.crypts;
    let mut i = 0usize;
    while i + 1 < patterns.len() { // expect bottom/up[/top] triplets
        let bottom_pat = &patterns[i];
        let up_pat = &patterns[i + 1];
        let top_pat = if i + 2 < patterns.len() { Some(&patterns[i + 2]) } else { None };

        // Bottom: all mushrooms in first pattern
        let mut bottom_world: Vec<BlockPos> = Vec::new();
        for b in bottom_pat.iter().filter(|b| matches!(b.block_id, Some(39 | 40))) {
            let rotated = rotate_block_pos(b, rotation);
            bottom_world.push(BlockPos { x: corner_pos.x + rotated.x, y: b.y, z: corner_pos.z + rotated.z });
        }

        // Up: prefer markers (block_id == 1), else all blocks in up pattern
        let mut up_world: Vec<BlockPos> = Vec::new();
        let mut up_src: Vec<&CryptBlock> = up_pat.iter().filter(|b| matches!(b.block_id, Some(1))).collect();
        if up_src.is_empty() {
            up_src = up_pat.iter().collect();
        }
        for b in up_src {
            let rotated = rotate_block_pos(b, rotation);
            up_world.push(BlockPos { x: corner_pos.x + rotated.x, y: b.y, z: corner_pos.z + rotated.z });
        }

        // Top: third pattern mushrooms if present; else derive from highest y among bottom
        let mut top_world: Vec<BlockPos> = Vec::new();
        if let Some(tp) = top_pat {
            for b in tp.iter().filter(|b| matches!(b.block_id, Some(39 | 40))) {
                let rotated = rotate_block_pos(b, rotation);
                top_world.push(BlockPos { x: corner_pos.x + rotated.x, y: b.y, z: corner_pos.z + rotated.z });
            }
        } else {
            let mush: Vec<&CryptBlock> = bottom_pat.iter().filter(|b| matches!(b.block_id, Some(39 | 40))).collect();
            if !mush.is_empty() {
                let max_y = mush.iter().map(|b| b.y).max().unwrap();
                for b in mush.into_iter().filter(|b| b.y == max_y) {
                    let rotated = rotate_block_pos(b, rotation);
                    top_world.push(BlockPos { x: corner_pos.x + rotated.x, y: b.y, z: corner_pos.z + rotated.z });
                }
            }
        }

        if !bottom_world.is_empty() && !up_world.is_empty() && !top_world.is_empty() {
            out.push(MushroomSets { bottom: bottom_world, top: top_world, up: up_world });
        }

        i += 3;
    }

    out
}

/// Vanilla's Nausea/Confusion status effect id.
const NAUSEA_EFFECT_ID: u8 = 9;
/// "Max nausea" per explicit request - vanilla Nausea doesn't have a dramatically different look
/// per level the way most effects do, but this is as high as makes sense to call "max".
const NAUSEA_AMPLIFIER: i8 = 4;
/// Upper-bound safety net matching the round trip's own 100-tick (5s) return timer exactly -
/// `clear_mushroom_up_effects` below always removes the effect explicitly the moment either
/// return path (top click or the automatic timeout) actually fires, so this duration should
/// never actually run out on its own; it's just insurance against some future path that
/// teleports the player back without going through the shared removal helper.
const NAUSEA_DURATION_TICKS: i32 = 100;

/// Per explicit request: going up through the mushroom shaft should feel like stepping through a
/// portal - max Nausea for the duration of the trip, plus the same real portal sound/particle
/// combo `teleport_maze.rs`'s own pad landings already use, played at the destination.
pub fn apply_mushroom_up_effects(player: &mut crate::server::player::player::Player, pos: crate::server::utils::dvec3::DVec3) {
    player.write_packet(&crate::net::protocol::play::clientbound::AddEffect {
        entity_id: crate::net::var_int::VarInt(player.entity_id),
        effect_id: NAUSEA_EFFECT_ID,
        amplifier: NAUSEA_AMPLIFIER,
        duration: crate::net::var_int::VarInt(NAUSEA_DURATION_TICKS),
        hide_particles: false,
    });
    play_mushroom_portal_effect(player, pos);
}

/// Ends the trip's Nausea early and plays the same portal sound/particles at the return spot -
/// called from both return paths (`MushroomTop`'s click handler and `tactical_insertion`'s
/// automatic-timeout path) so the effect always goes away exactly when the player actually
/// arrives back, regardless of which of the two ways they got there.
pub fn clear_mushroom_up_effects(player: &mut crate::server::player::player::Player, pos: crate::server::utils::dvec3::DVec3) {
    player.write_packet(&crate::net::protocol::play::clientbound::RemoveEffect {
        entity_id: crate::net::var_int::VarInt(player.entity_id),
        effect_id: NAUSEA_EFFECT_ID,
    });
    play_mushroom_portal_effect(player, pos);
}

fn play_mushroom_portal_effect(player: &mut crate::server::player::player::Player, pos: crate::server::utils::dvec3::DVec3) {
    player.write_packet(&crate::net::protocol::play::clientbound::SoundEffect {
        sound: crate::server::utils::sounds::Sounds::EndermenPortal.id(),
        pos_x: pos.x,
        pos_y: pos.y,
        pos_z: pos.z,
        volume: 1.0,
        pitch: 1.0,
    });
    player.write_packet(&crate::net::protocol::play::clientbound::Particles {
        particle_id: crate::server::utils::particles::ParticleTypes::Portal.get_id(),
        long_distance: true,
        x: pos.x as f32,
        y: pos.y as f32 + 0.5,
        z: pos.z as f32,
        offset_x: 0.3,
        offset_y: 0.5,
        offset_z: 0.3,
        speed: 0.0,
        count: 20,
    });
}

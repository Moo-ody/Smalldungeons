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
    let data = include_str!("Evensmallerdungeonsdata/room_data/Mushroom secret/Mushroomdata.json");
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



use once_cell::sync::Lazy;
use include_dir::include_dir;
use serde::Deserialize;
use std::collections::HashMap;

use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::utils::direction::Direction;

#[derive(Debug, Deserialize)]
struct RelativeCoordsFile {
    schema: Option<u32>,
    rooms: HashMap<String, RoomCryptsEntry>,
}

#[derive(Debug, Deserialize)]
struct RoomCryptsEntry {
    #[serde(default)]
    crypts: Vec<Vec<CryptBlock>>, // array of crypts, each a list of blocks
}

#[derive(Debug, Clone, Deserialize)]
pub struct CryptBlock {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    #[serde(default)]
    pub block_id: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct CryptPattern {
    pub blocks: Vec<CryptBlock>, // relative to room corner, canonical orientation (North)
}

#[derive(Debug, Clone)]
pub struct RoomCrypts {
    pub patterns: Vec<CryptPattern>,
}

static RELATIVE_1X1: Lazy<Option<RelativeCoordsFile>> = Lazy::new(|| {
    let data = include_str!("../../room_data/relativecoords/1x1crypts.json");
    serde_json::from_str::<RelativeCoordsFile>(data).ok()
});

static RELATIVE_1X2: Lazy<Option<RelativeCoordsFile>> = Lazy::new(|| {
    let data = include_str!("../../room_data/relativecoords/1x2crypts.json");
    serde_json::from_str::<RelativeCoordsFile>(data).ok()
});

static RELATIVE_REST: Lazy<Option<RelativeCoordsFile>> = Lazy::new(|| {
    let data = include_str!("../../room_data/relativecoords/rest.json");
    serde_json::from_str::<RelativeCoordsFile>(data).ok()
});

// Additional sources contributed later
static RELATIVE_CHAMBERS: Lazy<Option<RelativeCoordsFile>> = Lazy::new(|| {
    let data = include_str!("../../room_data/relativecoords/Chambers.json");
    serde_json::from_str::<RelativeCoordsFile>(data).ok()
});

static RELATIVE_L1X3: Lazy<Option<RelativeCoordsFile>> = Lazy::new(|| {
    let data = include_str!("../../room_data/relativecoords/Lroomsandonebythrees.json");
    serde_json::from_str::<RelativeCoordsFile>(data).ok()
});

// Optional extra file: crypts(1).json. Loaded if present in the directory bundle.
static RELATIVE_EXTRA: Lazy<Option<RelativeCoordsFile>> = Lazy::new(|| {
    // Embed the directory and try to fetch the file at runtime (path relative to crate root)
    static DIR: include_dir::Dir<'_> = include_dir!("../../room_data/relativecoords");
    if let Some(file) = DIR.get_file("crypts(1).json") {
        let contents = file.contents_utf8().unwrap_or("");
        serde_json::from_str::<RelativeCoordsFile>(contents).ok()
    } else {
        None
    }
});

pub fn get_room_crypts(shape: &str, room_name: &str) -> Option<RoomCrypts> {
    // Select primary source based on shape
    let primary = match shape {
        "1x1" | "1x1_E" | "1x1_X" | "1x1_I" | "1x1_L" | "1x1_3" => RELATIVE_1X1.as_ref(),
        "1x2" => RELATIVE_1X2.as_ref(),
        _ => RELATIVE_REST.as_ref(),
    };

    // Normalize room names to make matching more forgiving
    fn normalize(s: &str) -> String {
        s.to_lowercase()
            .replace('_', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
    let want = normalize(room_name);

    // Helper: find entry in a file by exact or normalized key
    fn find_entry<'a>(file: &'a RelativeCoordsFile, key: &str, want_norm: &str) -> Option<&'a RoomCryptsEntry> {
        if let Some(e) = file.rooms.get(key) { return Some(e); }
        file.rooms.iter().find(|(k, _)| normalize(k) == want_norm).map(|(_, v)| v)
    }

    let mut all_blocks: Vec<Vec<CryptBlock>> = Vec::new();

    if let Some(file) = primary {
        if let Some(entry) = find_entry(file, room_name, &want) {
            all_blocks.extend(entry.crypts.iter().cloned());
        }
    }
    if let Some(chambers) = RELATIVE_CHAMBERS.as_ref() {
        if let Some(entry) = find_entry(chambers, room_name, &want) {
            all_blocks.extend(entry.crypts.iter().cloned());
        }
    }
    if let Some(l1x3) = RELATIVE_L1X3.as_ref() {
        if let Some(entry) = find_entry(l1x3, room_name, &want) {
            all_blocks.extend(entry.crypts.iter().cloned());
        }
    }
    if let Some(extra) = RELATIVE_EXTRA.as_ref() {
        if let Some(entry) = find_entry(extra, room_name, &want) {
            all_blocks.extend(entry.crypts.iter().cloned());
        }
    }

    if all_blocks.is_empty() { return None; }

    let patterns = all_blocks.into_iter().map(|blocks| CryptPattern { blocks }).collect();
    Some(RoomCrypts { patterns })
}

pub fn rotate_block_pos(block: &CryptBlock, rotation: Direction) -> BlockPos {
    let bp = BlockPos { x: block.x, y: block.y, z: block.z };
    bp.rotate(rotation)
}

pub fn block_id_to_blocks(block_id: u16) -> Blocks {
    // block_id in JSON appears to be classic numeric id; we map to Air if unknown
    // Prefer using state id in future; for now set simple placeholder types
    match block_id {
        0 => Blocks::Air,
        1 => Blocks::Stone { variant: 0 },
        2 => Blocks::Grass,
        3 => Blocks::Dirt { variant: 0 },
        4 => Blocks::Cobblestone,
        5 => Blocks::WoodPlank { variant: 0 },
        43 => Blocks::DoubleStoneSlab { variant: crate::server::block::metadata::u3(0), seamless: false },
        44 => Blocks::StoneSlab { variant: crate::server::block::metadata::u3(0), top_half: false },
        89 => Blocks::GlowStone,
        98 => Blocks::StoneBrick { variant: 0 },
        109 => Blocks::StoneBrickStairs { direction: crate::server::block::block_parameter::StairDirection::North, top_half: false },
        _ => Blocks::Stone { variant: 0 },
    }
}


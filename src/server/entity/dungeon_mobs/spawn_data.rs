//! Parses `src/room_data/mobs/*.json` - one file per room, each a bag of observed mob
//! spawn/placement entries for that room (coordinates, equipment, etc). This module only
//! loads and indexes the raw data; picking a layout and spawning entities happens in
//! `spawner.rs`.

use include_dir::include_dir;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobRoomFile {
    pub room_name: String,
    #[serde(default)]
    pub total_visits: u32,
    #[serde(default)]
    pub spawns: Vec<MobSpawnJson>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobSpawnJson {
    /// Groups spawn entries that were observed together during the same dungeon run -
    /// i.e. one possible mob layout for the room. See `spawner::spawn_room_mobs`.
    pub run_index: u32,
    pub mob_type: String,
    #[serde(default)]
    pub is_starred: bool,
    pub full_name: String,
    pub rel_x: f64,
    pub rel_y: f64,
    pub rel_z: f64,
    #[serde(default)]
    pub yaw: f32,
    #[serde(default)]
    pub pitch: f32,
    #[serde(default)]
    pub equipment: HashMap<String, EquipmentJson>,
    /// Mojang profile "textures" property value, only present for `mobType: "player"` entries.
    #[serde(default)]
    pub skin: Option<String>,
    #[serde(default)]
    pub skin_signature: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquipmentJson {
    pub id: String,
    #[serde(default = "default_count")]
    pub count: i32,
    #[serde(default)]
    pub name: Option<String>,
    /// Leather armor dye color, packed RGB int.
    #[serde(default)]
    pub color: Option<i64>,
    /// Raw modern-format SNBT blob. Only `minecraft:custom_name`'s color is scraped out of
    /// this (see `equipment_convert::extract_custom_name_color`) - everything else about the
    /// item comes from the sibling fields above.
    #[serde(default)]
    pub nbt: Option<String>,
    /// Mojang profile "textures" property value, present on `minecraft:player_head` equipment.
    #[serde(default)]
    pub skull_texture: Option<String>,
}

fn default_count() -> i32 {
    1
}

/// Normalizes a room name for matching against a mob data file: lowercase with whitespace
/// and underscores stripped, so `"Blue Skulls"` and the file `blue_skulls.json` agree.
fn normalize_room_name(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

static MOB_ROOM_DATA: Lazy<HashMap<String, MobRoomFile>> = Lazy::new(|| {
    static DIR: include_dir::Dir<'_> = include_dir!("src/room_data/mobs");

    let mut map = HashMap::new();
    for file in DIR.files() {
        let Some(contents) = file.contents_utf8() else { continue };
        match serde_json::from_str::<MobRoomFile>(contents) {
            Ok(parsed) => {
                let key = normalize_room_name(&parsed.room_name);
                map.insert(key, parsed);
            }
            Err(e) => {
                println!("[dungeon_mobs] failed to parse {:?}: {e}", file.path());
            }
        }
    }
    map
});

/// Looks up the recorded mob spawn data for a room by its `RoomData.name`.
pub fn get_room_mob_spawns(room_name: &str) -> Option<&'static MobRoomFile> {
    MOB_ROOM_DATA.get(&normalize_room_name(room_name))
}

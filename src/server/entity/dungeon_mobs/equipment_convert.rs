//! Converts the modern-format equipment JSON found in room mob data into this server's
//! legacy (1.8-style) [`Equipment`]/[`ItemStack`] representation.

use crate::server::entity::dungeon_mobs::spawn_data::EquipmentJson;
use crate::server::entity::equipment::Equipment;
use crate::server::items::item_stack::ItemStack;
use std::collections::HashMap;

/// Builds an [`Equipment`] from a spawn entry's `equipment` map (JSON slot names
/// `mainhand`/`head`/`chest`/`legs`/`feet`).
pub fn convert_equipment(equipment: &HashMap<String, EquipmentJson>) -> Equipment {
    Equipment {
        main_hand: equipment.get("mainhand").map(convert_item),
        helmet: equipment.get("head").map(convert_item),
        chest: equipment.get("chest").map(convert_item),
        legs: equipment.get("legs").map(convert_item),
        boots: equipment.get("feet").map(convert_item),
        // Dungeon mobs never drop or lose their gear on hit, per Catacombs rules.
        no_loot_no_pickup: true,
        unbreakable: true,
    }
}

fn convert_item(json: &EquipmentJson) -> ItemStack {
    let mut stack = ItemStack::new(legacy_item_id(&json.id));
    stack.stack_size = json.count.clamp(1, 64) as i8;

    if json.id == "minecraft:player_head" {
        stack.metadata = 3; // legacy skull damage value for a player head
        if let Some(texture) = &json.skull_texture {
            stack.set_skull_owner(texture);
        }
    }

    if let Some(name) = &json.name {
        let color_code = json.nbt.as_deref()
            .and_then(extract_custom_name_color)
            .unwrap_or(WHITE_CODE);
        stack.set_display_name(&format!("{color_code}{name}"));
    }

    if let Some(color) = json.color {
        stack.set_dyed_color(color as i32);
    }

    stack.set_unbreakable(true);
    stack
}

/// Legacy (1.8) numeric item ids for the small, fixed set of items that appear across every
/// room's mob equipment. Falls back to Stone so an unmapped id is at least visible rather
/// than silently missing.
fn legacy_item_id(modern_id: &str) -> i16 {
    match modern_id {
        "minecraft:bone" => 352,
        "minecraft:bow" => 261,
        "minecraft:diamond_boots" => 313,
        "minecraft:diamond_chestplate" => 311,
        "minecraft:diamond_helmet" => 310,
        "minecraft:diamond_leggings" => 312,
        "minecraft:diamond_sword" => 276,
        "minecraft:fishing_rod" => 346,
        "minecraft:golden_boots" => 317,
        "minecraft:golden_chestplate" => 315,
        "minecraft:golden_helmet" => 314,
        "minecraft:golden_leggings" => 316,
        "minecraft:iron_sword" => 267,
        "minecraft:leather_boots" => 301,
        "minecraft:leather_chestplate" => 299,
        "minecraft:leather_helmet" => 298,
        "minecraft:leather_leggings" => 300,
        "minecraft:player_head" => 397,
        "minecraft:potion" => 373,
        "minecraft:stick" => 280,
        "minecraft:stone_sword" => 272,
        _ => 1,
    }
}

const WHITE_CODE: &str = "\u{a7}f";

/// The equipment JSON's `nbt` field is a raw modern-format SNBT blob; the only piece of it
/// worth digging out (everything else is already exposed as plain fields) is the color of
/// the `minecraft:custom_name` component, e.g. `"minecraft:custom_name":{color:"dark_purple",...}`.
/// This is a small targeted scrape, not a general SNBT parser.
fn extract_custom_name_color(nbt: &str) -> Option<&'static str> {
    let after_key = nbt.split("custom_name").nth(1)?;
    let after_color = after_key.split("color:\"").nth(1)?;
    let (color_name, _) = after_color.split_once('"')?;
    Some(mc_color_code(color_name))
}

fn mc_color_code(name: &str) -> &'static str {
    match name {
        "black" => "\u{a7}0",
        "dark_blue" => "\u{a7}1",
        "dark_green" => "\u{a7}2",
        "dark_aqua" => "\u{a7}3",
        "dark_red" => "\u{a7}4",
        "dark_purple" => "\u{a7}5",
        "gold" => "\u{a7}6",
        "gray" => "\u{a7}7",
        "dark_gray" => "\u{a7}8",
        "blue" => "\u{a7}9",
        "green" => "\u{a7}a",
        "aqua" => "\u{a7}b",
        "red" => "\u{a7}c",
        "light_purple" => "\u{a7}d",
        "yellow" => "\u{a7}e",
        "white" => "\u{a7}f",
        _ => WHITE_CODE,
    }
}

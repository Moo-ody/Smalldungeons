use std::collections::HashMap;
use once_cell::sync::Lazy;
use crate::server::player::terminal::{pane, Term, Terminal};
use rand::Rng;
use crate::net::protocol::play::serverbound::ClickWindow;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::utils::nbt::nbt::NBT;
use crate::server::utils::sounds::Sounds;

pub(crate) struct StartsWith;

const SIZE: usize = 45; // 9*5
pub(crate) const LETTERS: [&str; 10] = ["A", "B", "C", "G", "D", "M", "N", "R", "S", "T"];
static ITEM_MAP: Lazy<HashMap<i16, String>> = Lazy::new(|| {
    let file_content = include_str!("../../../room_data/minecraft/item_registry.json");
    serde_json::from_str(&file_content)
        .expect("Failed to parse JSON")
});

impl Term for StartsWith {
    fn click_slot(terminal: &mut Terminal, player: &mut Player, slot: usize, _packet: &ClickWindow) -> bool {
        let sloti8 = &(slot as i8);

        if terminal.solution.contains_key(sloti8) {
            let value = terminal.solution[sloti8];

            if value == 0 {
                let stack = terminal.get_slot_cloned(slot).unwrap();
                terminal.set_slot(create_item(stack.item, stack.metadata, &*get_item_name(stack.item), true), slot);
                terminal.play_sound(player, Sounds::Orb.id());
                terminal.solution.insert(*sloti8, 1);
                return Self::check(terminal);
            }
        }
        false
    }

    fn create(rand: i16) -> (Vec<Option<ItemStack>>, HashMap<i8, i8>) {
        let mut rng = rand::rng();
        let mut contents: Vec<Option<ItemStack>> = Vec::new();
        let mut map: HashMap<i8, i8> = HashMap::new();
        let letter = LETTERS[rand as usize % LETTERS.len()];
        for i in 0..SIZE {
            let row = i / 9;
            let col = i % 9;

            let result: ItemStack;
            if row >= 1 && row <= 3 && col >= 1 && col <= 7 {
                if rng.random_range(0..=7) + 10 == i {
                    result = get_letter_item_stack(false, letter);
                    map.insert(i as i8, 0); // false -> to be clicked
                } else if rng.random::<f64>() > 0.7 {
                    result = get_letter_item_stack(false, letter);
                    map.insert(i as i8, 0); // false -> to be clicked
                } else {
                    result = get_letter_item_stack(true, letter);
                }
            } else {
                result = pane(15, "");
            }
            contents.push(Some(result));
        }
        (contents, map)
    }

    fn check(terminal: &Terminal) -> bool {
        !terminal.solution.values().any(|&v| v == 0)
    }
}
fn random_item_id() -> i32 {
    let mut rng = rand::rng();
    rng.random_range(1..=431)
}
fn get_letter_item_stack(exclude: bool, letter: &str) -> ItemStack {
    let mut items: Vec<i16> = Vec::new();
    for (k, v) in ITEM_MAP.iter() {
        if v.starts_with(letter) != exclude {
            items.push(*k);
        }
    }
    if items.is_empty() {
        return create_item(0, 0, "§cError", false);
    }
    let r = rand::rng().random_range(0..items.len());
    create_item(items[r], 0, &*get_item_name(items[r]), false)
}
fn get_item_name(index: i16) -> String {
    ("§a".to_owned() + ITEM_MAP.get(&index).expect(&format!("Unable to get item {}", index))).to_string()
}
fn create_item(item: i16, metadata: i16, name: &str, enchanted: bool) -> ItemStack {
    ItemStack {
        item,
        stack_size: 1,
        metadata,
        tag_compound: if enchanted {
            Some(NBT::with_nodes(vec![
                NBT::compound("display", vec![
                    NBT::string("Name", name)
                ]),
                NBT::list("ench", 0, Vec::new())
            ]))
        } else {
            Some(NBT::with_nodes(vec![
                NBT::compound("display", vec![
                    NBT::string("Name", name)
                ])
            ]))
        },
    }
}
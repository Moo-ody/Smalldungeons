use std::collections::HashMap;
use once_cell::sync::Lazy;
use crate::server::player::terminal::{pane, Term, Terminal};
use rand::Rng;
use crate::net::protocol::play::serverbound::ClickWindow;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::utils::nbt::nbt::NBT;
use crate::server::utils::sounds::Sounds;

pub(crate) struct Select;

const SIZE: usize = 54; // 9*6
pub static ENUM_DYE: Lazy<HashMap<&'static i16, EnumDye>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(&0, EnumDye { damage: 15, name: "WHITE".to_string() });
    m.insert(&1, EnumDye { damage: 14, name: "ORANGE".to_string() });
    m.insert(&2, EnumDye { damage: 13, name: "MAGENTA".to_string() });
    m.insert(&3, EnumDye { damage: 12, name: "LIGHT BLUE".to_string() });
    m.insert(&4, EnumDye { damage: 11, name: "YELLOW".to_string() });
    m.insert(&5, EnumDye { damage: 10, name: "LIME".to_string() });
    m.insert(&6, EnumDye { damage: 9, name: "PINK".to_string() });
    m.insert(&7, EnumDye { damage: 8, name: "GRAY".to_string() });
    m.insert(&8, EnumDye { damage: 7, name: "SILVER".to_string() });
    m.insert(&9, EnumDye { damage: 6, name: "CYAN".to_string() });
    m.insert(&10, EnumDye { damage: 5, name: "PURPLE".to_string() });
    m.insert(&11, EnumDye { damage: 4, name: "BLUE".to_string() });
    m.insert(&12, EnumDye { damage: 3, name: "BROWN".to_string() });
    m.insert(&13, EnumDye { damage: 2, name: "GREEN".to_string() });
    m.insert(&14, EnumDye { damage: 1, name: "RED".to_string() });
    m.insert(&15, EnumDye { damage: 0, name: "BLACK".to_string() });
    m
});

pub static ITEMS: Lazy<HashMap<&'static i8, i16>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(&0, 35);
    m.insert(&1, 95);
    m.insert(&2, 159);
    m.insert(&3, 351);
    m
});

pub struct EnumDye {
    damage: i8,
    pub(crate) name: String,
}

impl Term for Select {
    fn click_slot(terminal: &mut Terminal, player: &mut Player, slot: usize, _packet: &ClickWindow) -> bool {
        let sloti8 = &(slot as i8);

        if terminal.solution.contains_key(sloti8) {
            let value = terminal.solution[sloti8];
            if value == 0 {
                let stack = terminal.get_slot_cloned(slot).unwrap();
                terminal.set_slot(create_item(stack.item, stack.metadata, true), slot);
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
        let guaranteed = get_random_guaranteed_slot();

        for i in 0..SIZE {
            let row = i / 9;
            let col = i % 9;

            let result: ItemStack;
            if row >= 1 && row <= 4 && col >= 1 && col <= 7 {
                let id = ITEMS[&rng.random_range(0..4)];
                if i == guaranteed{
                    result = if id == 351 { create_item(id, ENUM_DYE[&rand].damage.into(),false) } else { create_item(id, rand,false) };
                    map.insert(i as i8, 0); // false -> needs to be clicked
                } else {
                    if rng.random::<f64>() > 0.75 {
                        result = if id == 351 { create_item(id, ENUM_DYE[&rand].damage.into(),false) } else { create_item(id, rand,false) };
                        map.insert(i as i8, 0); // false -> needs to be clicked
                    } else {
                        let meta = random_wrong(rand);
                        result = if id == 351 { create_item(id, ENUM_DYE[&meta].damage.into(),false) } else { create_item(id, meta,false) };
                    }
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

fn get_random_guaranteed_slot() -> usize {
    let mut guaranteed_slots: Vec<usize> = Vec::new();

    guaranteed_slots.extend(10..=16);
    guaranteed_slots.extend(19..=25);
    guaranteed_slots.extend(28..=34);
    guaranteed_slots.extend(37..=43);

    let mut rng = rand::rng();
    let index = rng.random_range(0..guaranteed_slots.len());

    guaranteed_slots[index]
}
fn create_item(item: i16, metadata: i16, enchanted: bool) -> ItemStack {
    ItemStack {
        item,
        stack_size: 1,
        metadata,
        tag_compound: if enchanted {
            Some(NBT::with_nodes(vec![
                NBT::list("ench", 0, Vec::new())
            ]))
        } else {
            None
        },
    }
}
fn random_wrong(exclude: i16) -> i16 {
    let mut rng = rand::rng();
    let mut sel = rng.random_range(0..=15);
    while sel == exclude {
        sel = rng.random_range(0..=15);
    }
    sel
}
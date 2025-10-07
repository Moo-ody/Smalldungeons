use std::collections::HashMap;
use rand::Rng;
use crate::net::protocol::play::serverbound::{ClickMode, ClickWindow};
use crate::server::player::terminal::{pane, Term, Terminal};
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::utils::sounds::Sounds;

pub(crate) struct Rubix;

const SIZE: usize = 9*5;
const ORDER: [i16; 5] = [1, 4, 13, 11, 14];

impl Term for Rubix {
    fn click_slot(terminal: &mut Terminal, player: &mut Player, slot: usize, packet: &ClickWindow) -> bool {
        let sloti8 = &(slot as i8);

        if terminal.solution.contains_key(sloti8) {
            let stack = terminal.get_slot_cloned(slot).unwrap();
            let i = index_of(stack.metadata);
            let index;

            if packet.mode == ClickMode::NormalClick && packet.used_button == 1 {
                index = if stack.metadata == 1 { 4 } else { i - 1};
            } else {
                index = if stack.metadata == 14 { 0 } else { i + 1 };
            }
            let meta = ORDER[index as usize];

            terminal.set_slot(pane(meta, &*name_for_meta(meta)), slot);

            terminal.play_sound(player, Sounds::Orb.id());

            terminal.solution.insert(*sloti8, index as i8);
            return Self::check(terminal);
        }
        false
    }

    fn create(_rand: i16) -> (Vec<Option<ItemStack>>, HashMap<i8, i8>) {
        let mut contents: Vec<Option<ItemStack>> = Vec::new();
        let mut map: HashMap<i8, i8> = HashMap::new();

        for i in 0..SIZE {
            let result: ItemStack;
            let row = i / 9;
            let col = i % 9;

            if row >= 1 && row <= 3 && col >= 3 && col <= 5 {
                result = gen_pane();
                map.insert(i as i8, index_of(result.metadata) as i8);
            } else {
                result = pane(15, "");
            }
            contents.push(Some(result));
        }
        (contents, map)

    }

    fn check(terminal: &Terminal) -> bool {
        let mut iter = terminal.solution.values();
        if let Some(&first) = iter.next() {
            iter.all(|&v| v == first)
        } else {
            true
        }
    }
}
fn gen_pane() -> ItemStack {
    let mut rng = rand::rng();
    let x: f64 = rng.random();
    if x < 0.2 {
        pane(ORDER[0], &*name_for_meta(ORDER[0]))
    } else if x < 0.4 {
        pane(ORDER[1], &*name_for_meta(ORDER[1]))
    } else if x < 0.6 {
        pane(ORDER[2], &*name_for_meta(ORDER[2]))
    } else if x < 0.8 {
        pane(ORDER[3], &*name_for_meta(ORDER[3]))
    } else {
        pane(ORDER[4], &*name_for_meta(ORDER[4]))
    }
}
fn name_for_meta(meta: i16) -> String {
    match meta {
        1 => {
            "§aOrange".to_string()
        }
        4 => {
            "§aYellow".to_string()
        }
        13 => {
            "§aGreen".to_string()
        }
        11 => {
            "§aBlue".to_string()
        }
        _ => {
            "§aRed".to_string()
        }
    }
}
fn index_of(value: i16) -> i16 {
    if let Some(index) = ORDER.iter().position(|&x| x == value) {
        index as i16
    } else {
        -1
    }
}
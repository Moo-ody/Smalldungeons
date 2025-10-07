use std::collections::HashMap;
use crate::server::player::terminal::{pane, Term, Terminal};
use rand::Rng;
use crate::net::protocol::play::serverbound::ClickWindow;
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::utils::sounds::Sounds;

pub(crate) struct Panes;

const SIZE: usize = 45; // 9*5

impl Term for Panes {
    fn click_slot(terminal: &mut Terminal, player: &mut Player, slot: usize, _packet: &ClickWindow) -> bool {
        let sloti8 = &(slot as i8);

        if terminal.solution.contains_key(sloti8) {
            let value = terminal.solution[sloti8];

            if value == 1 {
                terminal.set_slot(pane(14, "§cOff"), slot);
                terminal.play_sound(player, Sounds::Orb.id());
                terminal.solution.insert(*sloti8, 0);
            } else {
                terminal.set_slot(pane(5, "§aOn"), slot);
                terminal.play_sound(player, Sounds::Orb.id());
                terminal.solution.insert(*sloti8, 1);
                return Self::check(terminal);
            }
        }
        false
    }

    fn create(_rand: i16) -> (Vec<Option<ItemStack>>, HashMap<i8, i8>) {
        let mut rng = rand::rng();
        let mut contents: Vec<Option<ItemStack>> = Vec::new();
        let mut map: HashMap<i8, i8> = HashMap::new();
        for i in 0..SIZE {
            let row = i / 9;
            let col = i % 9;

            let result: ItemStack;
            if row >= 1 && row <= 3 && col >= 2 && col <= 6 {
                let x: f64 = rng.random();
                if x > 0.75 {
                    result = pane(5, "§aOn");
                    map.insert(i as i8, 1); // true
                } else {
                    result = pane(14, "§cOff");
                    map.insert(i as i8, 0); // false
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
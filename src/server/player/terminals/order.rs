use std::collections::{HashMap, VecDeque};
use rand::prelude::SliceRandom;
use crate::net::protocol::play::serverbound::ClickWindow;
use crate::server::player::terminal::{Term, Terminal};
use crate::server::items::item_stack::ItemStack;
use crate::server::player::player::Player;
use crate::server::utils::nbt::nbt::NBT;
use crate::server::utils::sounds::Sounds;

pub(crate) struct Order;

const SIZE: usize = 9*4;

impl Term for Order {
    fn click_slot(terminal: &mut Terminal, player: &mut Player, slot: usize, _packet: &ClickWindow) -> bool {
        let sloti8 = &(slot as i8);

        if terminal.solution.contains_key(&0) && terminal.solution.get(&0).unwrap() == sloti8 {
            let stack = terminal.get_slot_cloned(slot);
            terminal.set_slot(pane(5, stack.unwrap().stack_size,""), slot);
            terminal.play_sound(player, Sounds::Orb.id());

            terminal.solution.remove(&0);

            let mut new_map = HashMap::new();
            for (k, v) in terminal.solution.iter() {
                new_map.insert(k - 1, *v);
            }
            terminal.solution = new_map;
            return Self::check(terminal);
        } else {
            player.send_message("§cWrong number!");
        }
        false
    }

    fn create(_rand: i16) -> (Vec<Option<ItemStack>>, HashMap<i8, i8>) {
        let mut rng = rand::rng();
        let mut contents: Vec<Option<ItemStack>> = Vec::new();
        let mut used: VecDeque<i8> = (1..=14).collect();
        let mut map: HashMap<i8, i8> = HashMap::new();

        let mut vec: Vec<_> = used.into_iter().collect();
        vec.shuffle(&mut rng);
        used = vec.into_iter().collect();

        for i in 0..SIZE {
            let row = i / 9;
            let col = i % 9;

            let result: ItemStack;

            if (row == 1 || row == 2) && col >= 1 && col <= 7 {
                let amount = used.pop_front().unwrap();
                result = pane(14, amount, "");

                map.insert(amount - 1, i as i8);
            } else {
                result = pane(15,1,"");
            }
            contents.push(Some(result));
        }
        (contents, map)
    }

    fn check(terminal: &Terminal) -> bool {
        terminal.solution.is_empty()
    }
}

fn pane(meta: i16, stack_size: i8, name: &str) -> ItemStack {
    ItemStack {
        item: 160,
        stack_size,
        metadata: meta,
        tag_compound: Some(NBT::with_nodes(vec![
            NBT::compound("display", vec![
                NBT::string("Name", name)
            ])
        ])),
    }
}
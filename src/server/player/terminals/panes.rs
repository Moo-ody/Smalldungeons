use crate::server::player::terminal::{Term, Terminal};
use rand::Rng;
use crate::server::items::item_stack::ItemStack;
use crate::server::utils::nbt::nbt::NBT;

pub(crate) struct Panes;

const SIZE: usize = 45; // 9*5

impl Term for Panes { // i am in pain, maybe i should have learned rust before doing this
    fn click_slot(terminal: &mut Terminal, slot: usize) -> bool {
        let row = slot / 9;
        let col = slot % 9;
        if row >= 1 && row <= 3 && col >= 2 && col <= 6 {

            let option = terminal.get_slot_cloned(slot);
            if option.is_none() { return false; }
            let item = option.unwrap();

            if item.metadata == 14 { // holy schizophrenia
                terminal.set_slot(pane(5, "§aOn"), slot);
                return Self::check(terminal);
            } else {
                terminal.set_slot(pane(14, "§cOff"), slot);
            }
        }
        false
    }

    fn create() -> Vec<Option<ItemStack>> {
        let mut rng = rand::rng();
        let mut contents: Vec<Option<ItemStack>> = Vec::new();
        for i in 0..SIZE {
            let row = i / 9;
            let col = i % 9;

            let result: ItemStack;
            if row >= 1 && row <= 3 && col >= 2 && col <= 6 {
                let x: f64 = rng.random();
                if x > 0.75 {
                    result = pane(5, "§aOn");
                } else {
                    result = pane(14, "§cOff");
                }
            } else {
                result = pane(15, "");
            }
            contents.push(Option::from(result));
        }
        contents
    }

    fn check(terminal: &mut Terminal) -> bool {
        for i in 0..SIZE {
            let row = i / 9;
            let col = i % 9;
            if row >= 1 && row <= 3 && col >= 2 && col <= 6 {
                let item = terminal.get_slot_cloned(i);
                if item.is_some() && item.clone().unwrap().metadata == 14 { // red
                    return false;
                }
            }
        }
        true
    }
}

fn pane(meta: i16, name: &str) -> ItemStack {
    ItemStack {
        item: 160,
        stack_size: 1,
        metadata: meta,
        tag_compound: Some(NBT::with_nodes(vec![
            NBT::compound("display", vec![
                NBT::string("Name", name)
            ])
        ])),
    }
}
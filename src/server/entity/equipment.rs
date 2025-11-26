use crate::server::items::item_stack::ItemStack;

/// Equipment component attached to living mobs (no offhand)
#[derive(Clone, Debug, Default)]
pub struct Equipment {
    pub main_hand: Option<ItemStack>,
    pub helmet:    Option<ItemStack>,
    pub chest:     Option<ItemStack>,
    pub legs:      Option<ItemStack>,
    pub boots:     Option<ItemStack>,
    pub no_loot_no_pickup: bool, // enforce Dungeons rules
    pub unbreakable: bool,       // mark all items unbreakable
}

impl Equipment {
    /// Apply unbreakable NBT to all equipped items that have NBT
    pub fn apply_unbreakable(&mut self) {
        if !self.unbreakable {
            return;
        }

        for slot in [
            &mut self.main_hand,
            &mut self.helmet,
            &mut self.chest,
            &mut self.legs,
            &mut self.boots,
        ] {
            if let Some(stack) = slot.as_mut() {
                stack.set_unbreakable(true);
            }
        }
    }

    /// Check if this equipment setup follows Dungeons rules
    pub fn follows_dungeons_rules(&self) -> bool {
        self.no_loot_no_pickup
    }
}

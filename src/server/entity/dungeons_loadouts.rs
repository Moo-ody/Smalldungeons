use crate::server::items::item_stack::{ItemStack, ItemStackExt};

/// Dungeons presets for equipped zombies
pub mod dungeons_loadouts {
    use super::*;

    pub fn zombie_commander() -> crate::server::entity::equipment::Equipment {
        crate::server::entity::equipment::Equipment {
            helmet:    Some(ItemStack::new(298).leather_rgb(190, 0, 0).unbreakable()), // Leather Helmet
            chest:     Some(ItemStack::new(299).leather_rgb(190, 0, 0).unbreakable()), // Leather Chestplate
            legs:      Some(ItemStack::new(300).leather_rgb(170, 0, 0).unbreakable()), // Leather Leggings
            boots:     Some(ItemStack::new(301).leather_rgb(170, 0, 0).unbreakable()), // Leather Boots
            main_hand: Some(ItemStack::new(346).unbreakable()), // Fishing Rod
            no_loot_no_pickup: true,
            unbreakable: true,
        }
    }

    pub fn zombie_grunt() -> crate::server::entity::equipment::Equipment {
        crate::server::entity::equipment::Equipment {
            helmet:    None,
            chest:     Some(ItemStack::new(299).leather_rgb(60, 60, 60).unbreakable()), // Leather Chestplate
            legs:      Some(ItemStack::new(300).leather_rgb(50, 50, 50).unbreakable()), // Leather Leggings
            boots:     Some(ItemStack::new(301).leather_rgb(50, 50, 50).unbreakable()), // Leather Boots
            main_hand: Some(ItemStack::new(272).unbreakable()), // Stone Sword
            no_loot_no_pickup: true,
            unbreakable: true,
        }
    }

    pub fn zombie_custom() -> crate::server::entity::equipment::Equipment {
        crate::server::entity::equipment::Equipment {
            helmet:    Some(ItemStack::new(298).leather_rgb(213, 18, 48).unbreakable()), // Leather Helmet - #D51230
            chest:     Some(ItemStack::new(299).leather_rgb(213, 18, 48).unbreakable()), // Leather Chestplate - #D51230
            legs:      Some(ItemStack::new(300).leather_rgb(213, 18, 48).unbreakable()), // Leather Leggings - #D51230
            boots:     Some(ItemStack::new(301).leather_rgb(213, 18, 48).unbreakable()), // Leather Boots - #D51230
            main_hand: Some(ItemStack::new(346).unbreakable()), // Fishing Rod
            no_loot_no_pickup: true,
            unbreakable: true,
        }
    }
}

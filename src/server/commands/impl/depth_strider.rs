//! Testing convenience: gives the player Diamond Boots enchanted with Depth Strider III so
//! they can move through water/lava quickly while trying out mob swimming behavior.

use crate::server::commands::argument::Argument;
use crate::server::commands::command::CommandMetadata;
use crate::server::commands::outcome::Outcome;
use crate::server::items::Item;
use crate::server::player::inventory::ItemSlot;
use crate::server::player::player::Player;
use crate::server::world::World;

/// Boots armor slot in the player's own inventory container (0=craft result, 1-4=craft grid,
/// 5=helmet, 6=chestplate, 7=leggings, 8=boots).
const BOOTS_SLOT: usize = 8;

pub struct DepthStrider;

impl CommandMetadata for DepthStrider {
    const NAME: &'static str = "depthstrider";

    fn run(_: &mut World, player: &mut Player, _: &[&str]) -> anyhow::Result<Outcome> {
        player.inventory.set_slot(ItemSlot::Filled(Item::DepthStriderBoots, 1), BOOTS_SLOT);
        player.sync_inventory();
        Ok(Outcome::Success)
    }

    fn arguments(_: &mut World, _: &mut Player) -> Vec<Argument> {
        Vec::new()
    }
}

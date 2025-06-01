use crate::server::items::etherwarp::handle_etherwarp;
use crate::server::player::Player;

pub mod item_stack;
mod etherwarp;

/// List of items available to be used
/// TODO, more
#[derive(Copy, Debug, Clone)]
pub enum Item {
    AspectOfTheVoid,
    SpiritSceptre,
}

impl Item {

    pub fn handle_right_click(&self, player: &Player) -> anyhow::Result<()> {
        match self {
            Item::AspectOfTheVoid => {
                println!("HELLO");
                let server = &player.server_mut();
                let world = &server.world;
                let entity = player.get_entity(world)?;
                if player.is_sneaking {
                    handle_etherwarp(player, &server.network_tx, world, entity)?
                }

                // let pos = raycast_first_solid_block(world, entity, 60.0);
                // println!("pos raycasted {:?}", pos)
            }
            Item::SpiritSceptre => {
                // spawn bats, they copy yaw and pitch, idk the speed or whatever but
                // when they hit a solid block they blow up in like 10 block radius (or square) or something
            }
        }
        Ok(())
    }

}

// static items for cooldown sharing, and stuff?
pub static ASPECT_OF_THE_VOID: Item = Item::AspectOfTheVoid;
pub static SPIRIT_SCEPTRE: Item = Item::SpiritSceptre;
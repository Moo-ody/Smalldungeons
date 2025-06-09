use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::SendPacket;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::Entity;
use crate::server::server::Server;
use crate::server::utils::direction::Direction;
use crate::server::world::World;

pub struct Crusher {
    pub block_pos: BlockPos,
    pub direction: Direction,
    pub width: i32,
    pub height: i32,
    pub max_length: u16,
    tick_per_block: u16,
    pause_duration: u16,

    current_length: u16,
    tick: u16,

    is_paused: bool,
    is_reversed: bool,
}

impl Crusher {

    pub fn new(
        block_pos: BlockPos,
        direction: Direction,
        width: i32,
        height: i32,
        max_length: u16,
        tick_per_block: u16,
        pause_duration: u16,
    ) -> Crusher {
        Crusher {
            block_pos,
            direction,
            width,
            height,
            max_length,
            tick_per_block,
            pause_duration,
            current_length: 0,
            tick: 0,
            is_paused: false,
            is_reversed: false,
        }
    }

    pub fn tick(&mut self, server: &mut Server) {
        self.tick += 1;

        let world = &mut server.world;

        if self.is_paused {
            if self.tick == self.pause_duration {
                self.is_reversed = !self.is_reversed;
                self.is_paused = false;
                self.tick = 0;
            }
        } else {
            if self.tick % self.tick_per_block == 0 {
                let (dx, dz) = self.direction.get_offset();
                
                if !self.is_reversed {
                    let x = self.block_pos.x + (self.current_length as i32 * dx);
                    let y = self.block_pos.y;
                    let z = self.block_pos.z + (self.current_length as i32 * dz);
                    self.set_blocks(world, Blocks::PolishedGranite, x, y, z, dx, dz);

                    for (id, player) in &server.players {
                        let entity = player.get_entity_mut(world).unwrap();

                        if self.is_in_way(entity, x, y, z) {
                            PositionLook {
                                x: entity.pos.x + dx as f64,
                                y: entity.pos.y,
                                z: entity.pos.z + dz as f64,
                                yaw: entity.yaw,
                                pitch: entity.pitch,
                                flags: 0,
                            }.send_packet(*id, &server.network_tx).unwrap();
                        }
                    }
                } else {
                    let x = self.block_pos.x + ((self.max_length - self.current_length) as i32 * dx);
                    let y = self.block_pos.y;
                    let z = self.block_pos.z + ((self.max_length - self.current_length) as i32 * dz);
                    self.set_blocks(world, Blocks::Air, x, y, z, dx, dz)
                }
                self.current_length += 1;
            }
            if self.current_length == self.max_length {
                if self.pause_duration != 0 {
                    self.is_paused = true
                } else {
                    self.is_reversed = !self.is_reversed
                }
                self.tick = 0;
                self.current_length = 0;
            }
        }
    }

    fn set_blocks(&self, world: &mut World, block: Blocks, x: i32, y: i32, z: i32, dx: i32, dz: i32) {
        for w in 0..self.width {
            for h in 0..self.height {
                world.set_block_at(
                    block,
                    x + (w * dz),
                    y + h,
                    z + (w * dx)
                );
            }
        }
    }

    fn is_in_way(&self, entity: &Entity, x: i32, y: i32, z: i32) -> bool {
        let (x_offset, z_offset) = match self.direction {
            Direction::North => (1, 0),
            Direction::East => (0, 0),
            Direction::South => (0, 0),
            Direction::West => (0, 1),
        };
        let (width, length) = match self.direction {
            Direction::North => (-self.width, 1),
            Direction::East => (1, self.width),
            Direction::South => (self.width, 1),
            Direction::West => (1, -self.width),
        };
        entity.is_in_box_i32(x + x_offset, y, z + z_offset, width, self.height, length)
    }
}
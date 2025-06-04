use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::SendPacket;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::server::Server;
use crate::server::utils::direction::Direction;

pub struct Crusher {
    pub position: BlockPos,
    pub direction: Direction,
    pub max_length: u16,
    pub length: u16,

    pub tick: u16,
    pub tick_per_block: u16,
    pub pause_duration: u16,

    pub is_paused: bool,
    pub is_reversed: bool,
}

impl Crusher {
    pub fn tick(&mut self, server: &mut Server) {
        self.tick += 1;

        let mut world = &mut server.world;

        if self.is_paused {
            if self.tick == self.pause_duration {
                self.is_reversed = !self.is_reversed;
                self.is_paused = false;
                self.tick = 0;
            }
        } else {
            if self.tick % self.tick_per_block == 0 {
                if !self.is_reversed {
                    let block_x = self.position.x + self.length as i32;
                    let block_y = self.position.y;
                    let block_z = self.position.z;
                    world.set_block_at(Blocks::PolishedDiorite, block_x, block_y, block_z);

                    for (id, player) in &server.players {
                        let entity = player.get_entity_mut(world).unwrap();

                        let px = entity.pos.x;
                        let py = entity.pos.y;
                        let pz = entity.pos.z;

                        if  px >= block_x as f64 && px < (block_x + 1) as f64 &&
                            py >= block_y as f64 && py < (block_y + 1) as f64 &&
                            pz >= block_z as f64 && pz < (block_z + 1) as f64
                        {
                            PositionLook {
                                x: px + 1.0,
                                y: py,
                                z: pz,
                                yaw: entity.yaw,
                                pitch: entity.pitch,
                                flags: 0,
                            }.send_packet(*id, &server.network_tx).unwrap();
                        }
                    }
                } else {
                    world.set_block_at(
                        Blocks::Air,
                        self.position.x + (self.max_length - self.length) as i32,
                        self.position.y,
                        self.position.z
                    );
                }
                self.length += 1;
            }
            if self.length == self.max_length {
                if self.pause_duration != 0 {
                    self.is_paused = true
                } else {
                    self.is_reversed = !self.is_reversed
                }
                self.tick = 0;
                self.length = 0;
            }
        }
    }
}
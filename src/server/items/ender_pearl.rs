use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::player::player::Player;
use crate::server::world::World;
use crate::server::block::blocks::Blocks;
use crate::server::utils::dvec3::DVec3;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::SendPacket;
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::ClientId;
use anyhow::Result;

/// Simple vanilla ender pearl implementation
pub struct PearlEntityImpl {
    thrower_client_id: ClientId,
    velocity: DVec3,
}

impl PearlEntityImpl {
    pub fn new(thrower_client_id: ClientId, velocity: DVec3) -> Self {
        Self { 
            thrower_client_id, 
            velocity 
        }
    }

    /// Vanilla collision detection - just check if the block is solid
    fn check_collision(&self, world: &World, pos: DVec3) -> bool {
        let block_x = pos.x.floor() as i32;
        let block_y = pos.y.floor() as i32;
        let block_z = pos.z.floor() as i32;
        
        let block = world.get_block_at(block_x, block_y, block_z);
        
        // Vanilla just checks if block is not air
        block != Blocks::Air
    }
}

impl EntityImpl for PearlEntityImpl {
    fn spawn(&mut self, entity: &mut Entity) {
        // Set initial velocity
        entity.velocity = self.velocity;
    }

    fn tick(&mut self, entity: &mut Entity) {
        // Vanilla physics
        let gravity = -0.03;
        entity.velocity.y += gravity;
        entity.velocity.x *= 0.99;
        entity.velocity.y *= 0.99;
        entity.velocity.z *= 0.99;

        // Move pearl
        let next_pos = DVec3::new(
            entity.position.x + entity.velocity.x,
            entity.position.y + entity.velocity.y,
            entity.position.z + entity.velocity.z,
        );

        // Check collision
        if self.check_collision(entity.world_mut(), next_pos) {
            // Pearl hit something - teleport player
            if let Some(player) = entity.world_mut().players.get(&self.thrower_client_id) {
                // Get block coordinates where pearl landed
                let block_x = next_pos.x.floor() as i32;
                let block_z = next_pos.z.floor() as i32;
                
                // Center player on block X/Z, use pearl Y for feet
                let final_x = block_x as f64 + 0.5;
                let final_y = next_pos.y;
                let final_z = block_z as f64 + 0.5;

                let _ = PositionLook {
                    x: final_x,
                    y: final_y,
                    z: final_z,
                    yaw: 0.0,
                    pitch: 0.0,
                    flags: 24, // keep yaw/pitch, set absolute xyz
                }.send_packet(player.client_id, &player.network_tx);
            }

            // Despawn pearl
            entity.world_mut().despawn_entity(entity.id);
            return;
        }

        // No collision - update position
        entity.position = next_pos;

        // Lifetime limit
        if entity.ticks_existed > 200 {
            entity.world_mut().despawn_entity(entity.id);
        }
    }
}

pub fn on_right_click(player: &mut Player) -> Result<()> {
    let eye_height = 1.62;
    let eye_pos = player.position + DVec3::new(0.0, eye_height, 0.0);

    // Convert yaw/pitch to direction
    let yaw_rad = (player.yaw as f64).to_radians();
    let pitch_rad = (player.pitch as f64).to_radians();
    let dir = DVec3::new(
        -pitch_rad.cos() * yaw_rad.sin(),
        -pitch_rad.sin(),
        pitch_rad.cos() * yaw_rad.cos(),
    );
    let dir = dir.normalize();

    let velocity = DVec3::new(dir.x * 1.5, dir.y * 1.5, dir.z * 1.5);
    let spawn_pos = eye_pos + DVec3::new(dir.x * 0.2, dir.y * 0.2, dir.z * 0.2);

    player.world_mut().spawn_entity(
        spawn_pos,
        EntityMetadata::new(EntityVariant::EnderPearl),
        PearlEntityImpl::new(player.client_id, velocity),
    )?;

    Ok(())
}

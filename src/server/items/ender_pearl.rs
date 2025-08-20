use crate::net::packets::client_bound::entity::entity_velocity::EntityVelocity;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::SendPacket;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::{ClientId, Player};
use crate::server::block::blocks::Blocks;
use crate::server::utils::dvec3::DVec3;
use anyhow::Result;

/// Simple implementation for a thrown ender pearl
pub struct PearlEntityImpl {
    thrower_client_id: ClientId,
    velocity: DVec3,
}

impl PearlEntityImpl {
    pub fn new(thrower_client_id: ClientId, velocity: DVec3) -> Self {
        Self { thrower_client_id, velocity }
    }
}

impl EntityImpl for PearlEntityImpl {
    fn spawn(&mut self, entity: &mut Entity) {
        // Inform clients of initial motion so the projectile animates
        for player in entity.world_mut().players.values() {
            let _ = player.send_packet(EntityVelocity {
                entity_id: entity.id,
                motion_x: self.velocity.x,
                motion_y: self.velocity.y,
                motion_z: self.velocity.z,
            });
        }
        entity.velocity = self.velocity;
    }

    fn tick(&mut self, entity: &mut Entity) {
        // Basic projectile physics
        let gravity = -0.03;
        self.velocity.y += gravity;
        self.velocity.x *= 0.99;
        self.velocity.y *= 0.99;
        self.velocity.z *= 0.99;

        // Swept movement with discrete steps to detect first collision face
        let start = entity.position;
        let end = DVec3::new(
            start.x + self.velocity.x,
            start.y + self.velocity.y,
            start.z + self.velocity.z,
        );
        let max_delta = self.velocity.x.abs().max(self.velocity.y.abs()).max(self.velocity.z.abs());
        let steps = (max_delta / 0.2).ceil().max(1.0) as i32; // step ~0.2 blocks
        let step = DVec3::new(
            (end.x - start.x) / steps as f64,
            (end.y - start.y) / steps as f64,
            (end.z - start.z) / steps as f64,
        );

        let mut collided = false;
        let mut impact_axis: Option<char> = None; // 'x', 'y', 'z'
        let mut impact_block = (0, 0, 0);
        let mut current = start;

        for _ in 0..steps {
            let prev = current;
            current = DVec3::new(prev.x + step.x, prev.y + step.y, prev.z + step.z);

            let bx = current.x.floor() as i32;
            let by = current.y.floor() as i32;
            let bz = current.z.floor() as i32;
            let world = entity.world_mut();
            let block = world.get_block_at(bx, by, bz);
            if !is_block_passable_for_pearl(block) {
                // Determine axis of impact by seeing which cell boundary was crossed
                let p_bx = prev.x.floor() as i32;
                let p_by = prev.y.floor() as i32;
                let p_bz = prev.z.floor() as i32;
                impact_axis = Some(if bx != p_bx && self.velocity.x.abs() >= self.velocity.y.abs().max(self.velocity.z.abs()) {
                    'x'
                } else if by != p_by && self.velocity.y.abs() >= self.velocity.x.abs().max(self.velocity.z.abs()) {
                    'y'
                } else if bz != p_bz {
                    'z'
                } else if bx != p_bx {
                    'x'
                } else if by != p_by {
                    'y'
                } else {
                    'z'
                });
                impact_block = (bx, by, bz);
                collided = true;
                break;
            }
        }

        if collided {
            let (bx, by, bz) = impact_block;
            let world = entity.world_mut();
            if let Some(player) = world.players.get(&self.thrower_client_id) {
                let (tx, ty, tz) = match impact_axis.unwrap_or('y') {
                    'x' => {
                        // Snap to block center horizontally on impact
                        (bx as f64 + 0.5, current.y, bz as f64 + 0.5)
                    }
                    'z' => {
                        (bx as f64 + 0.5, current.y, bz as f64 + 0.5)
                    }
                    'y' => {
                        if self.velocity.y < 0.0 {
                            // Land on top of block
                            (bx as f64 + 0.5, by as f64 + 1.0, bz as f64 + 0.5)
                        } else {
                            // Clip head slightly into roof
                            (bx as f64 + 0.5, by as f64 - 0.2, bz as f64 + 0.5)
                        }
                    }
                    _ => (bx as f64 + 0.5, current.y, bz as f64 + 0.5),
                };

                let _ = PositionLook {
                    x: tx,
                    y: ty,
                    z: tz,
                    yaw: 0.0,
                    pitch: 0.0,
                    // keep yaw/pitch, set absolute xyz
                    flags: 24,
                }.send_packet(player.client_id, &player.network_tx);
            }
            world.despawn_entity(entity.id);
            return;
        }

        // No collision this tick: apply full movement
        entity.velocity = self.velocity;
        entity.position = current;

        // Lifetime limit to avoid lingering forever
        if entity.ticks_existed > 200 {
            entity.world_mut().despawn_entity(entity.id);
        }
    }
}

#[inline]
fn is_block_passable_for_pearl(block: Blocks) -> bool {
    match block {
        Blocks::Air
        | Blocks::FlowingWater { .. }
        | Blocks::StillWater { .. }
        | Blocks::FlowingLava { .. }
        | Blocks::Lava { .. }
        | Blocks::Tallgrass { .. }
        | Blocks::Deadbush
        | Blocks::Torch { .. }
        | Blocks::UnlitRedstoneTorch { .. }
        | Blocks::RedstoneTorch { .. }
        | Blocks::Redstone { .. }
        | Blocks::YellowFlower
        | Blocks::RedFlower { .. }
        | Blocks::Vine { .. }
        | Blocks::Fire
        | Blocks::Lilypad
        | Blocks::Carpet { .. }
        | Blocks::SnowLayer { .. }
        | Blocks::Skull { .. }
        | Blocks::FlowerPot { .. }
        | Blocks::RedstoneComparator { .. }
        | Blocks::PoweredRedstoneComparator { .. }
        | Blocks::RedstoneRepeater { .. }
        | Blocks::PoweredRedstoneRepeater { .. }
        | Blocks::Rail { .. }
        | Blocks::PoweredRail { .. }
        | Blocks::DetectorRail { .. }
        | Blocks::DaylightSensor { .. }
        | Blocks::InvertedDaylightSensor { .. }
        | Blocks::Ladder { .. }
        | Blocks::Trapdoor { open: true, .. }
        | Blocks::IronTrapdoor { open: true, .. }
        | Blocks::SpruceFenceGate { open: true, .. }
        | Blocks::BirchFenceGate { open: true, .. }
        | Blocks::JungleFenceGate { open: true, .. }
        | Blocks::DarkOakFenceGate { open: true, .. }
        | Blocks::AcaciaFenceGate { open: true, .. } => true,
        _ => false,
    }
}

pub fn on_right_click(player: &mut Player) -> Result<()> {
    let eye_height = 1.62; // player eye height in blocks
    let eye_pos = player.position + DVec3::new(0.0, eye_height, 0.0);

    // Convert yaw/pitch (degrees) to a forward direction vector
    let yaw_rad = (player.yaw as f64).to_radians();
    let pitch_rad = (player.pitch as f64).to_radians();
    let dir = DVec3::new(
        -pitch_rad.cos() * yaw_rad.sin(),
        -pitch_rad.sin(),
        pitch_rad.cos() * yaw_rad.cos(),
    );
    let dir = dir.normalize();

    let velocity = DVec3::new(dir.x * 1.5, dir.y * 1.5, dir.z * 1.5); // Vanilla-ish speed
    let spawn_pos = eye_pos + DVec3::new(dir.x * 0.2, dir.y * 0.2, dir.z * 0.2); // slight offset in front of player

    player.world_mut().spawn_entity(
        spawn_pos,
        EntityMetadata::new(EntityVariant::EnderPearl),
        PearlEntityImpl::new(player.client_id, velocity),
    )?;

    Ok(())
}

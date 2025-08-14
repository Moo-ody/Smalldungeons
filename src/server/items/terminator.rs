use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::net::packets::client_bound::entity::entity_velocity::EntityVelocity;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::SendPacket;
use std::f64::consts::PI;

pub fn on_right_click(player: &mut Player) -> anyhow::Result<()> {
    let eye_height = 1.62; // player eye height in blocks
    let eye_pos = player.position + DVec3::new(0.0, eye_height, 0.0);

    // Convert yaw/pitch (degrees) to a forward direction vector
    let yaw_rad = (player.yaw as f64).to_radians();
    let pitch_rad = (player.pitch as f64).to_radians();
    let base_dir = DVec3::new(
        -pitch_rad.cos() * yaw_rad.sin(),
        -pitch_rad.sin(),
        pitch_rad.cos() * yaw_rad.cos(),
    );
    let base_dir = base_dir.normalize();

    // Spawn 3 arrows in a spread pattern
    let spread_angles = [-0.1, 0.0, 0.1]; // Small spread for volley effect
    
    for &spread in &spread_angles {
        // Calculate direction with spread
        let spread_yaw = yaw_rad + spread;
        let dir = DVec3::new(
            -pitch_rad.cos() * spread_yaw.sin(),
            -pitch_rad.sin(),
            pitch_rad.cos() * spread_yaw.cos(),
        );
        let dir = dir.normalize();

        let velocity = DVec3::new(dir.x * 2.0, dir.y * 2.0, dir.z * 2.0); // Fast arrow speed
        let spawn_pos = eye_pos + DVec3::new(dir.x * 0.3, dir.y * 0.3, dir.z * 0.3); // slight offset in front of player

        player.world_mut().spawn_entity(
            spawn_pos,
            EntityMetadata::new(EntityVariant::Arrow),
            ArrowEntityImpl::new(velocity),
        )?;
    }

    Ok(())
}

/// Arrow entity implementation for Terminator bow
#[derive(Debug)]
pub struct ArrowEntityImpl {
    velocity: DVec3,
}

impl ArrowEntityImpl {
    pub fn new(velocity: DVec3) -> Self {
        Self { velocity }
    }
}

impl EntityImpl for ArrowEntityImpl {
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
        let gravity = -0.05; // Less gravity than ender pearl for arrows
        self.velocity.y += gravity;
        self.velocity.x *= 0.99;
        self.velocity.y *= 0.99;
        self.velocity.z *= 0.99;

        // Simple collision detection
        let start = entity.position;
        let end = DVec3::new(
            start.x + self.velocity.x,
            start.y + self.velocity.y,
            start.z + self.velocity.z,
        );

        // Check if we hit a solid block
        let world = entity.world_mut();
        let bx = end.x.floor() as i32;
        let by = end.y.floor() as i32;
        let bz = end.z.floor() as i32;
        
        let block = world.get_block_at(bx, by, bz);
        if !is_block_passable_for_arrow(block) {
            // Hit a solid block, despawn arrow
            world.despawn_entity(entity.id);
            return;
        }

        // No collision this tick: apply full movement
        entity.velocity = self.velocity;
        entity.position = end;

        // Lifetime limit to avoid lingering forever
        if entity.ticks_existed > 300 { // Longer lifetime for arrows
            entity.world_mut().despawn_entity(entity.id);
        }
    }
}

#[inline]
fn is_block_passable_for_arrow(block: crate::server::block::blocks::Blocks) -> bool {
    match block {
        crate::server::block::blocks::Blocks::Air
        | crate::server::block::blocks::Blocks::FlowingWater { .. }
        | crate::server::block::blocks::Blocks::StillWater { .. }
        | crate::server::block::blocks::Blocks::FlowingLava { .. }
        | crate::server::block::blocks::Blocks::Lava { .. }
        | crate::server::block::blocks::Blocks::Tallgrass { .. }
        | crate::server::block::blocks::Blocks::Deadbush
        | crate::server::block::blocks::Blocks::Torch { .. }
        | crate::server::block::blocks::Blocks::UnlitRedstoneTorch { .. }
        | crate::server::block::blocks::Blocks::RedstoneTorch { .. }
        | crate::server::block::blocks::Blocks::Redstone { .. }
        | crate::server::block::blocks::Blocks::YellowFlower
        | crate::server::block::blocks::Blocks::RedFlower { .. }
        | crate::server::block::blocks::Blocks::Vine { .. }
        | crate::server::block::blocks::Blocks::Fire
        | crate::server::block::blocks::Blocks::Lilypad
        | crate::server::block::blocks::Blocks::Carpet { .. }
        | crate::server::block::blocks::Blocks::SnowLayer { .. }
        | crate::server::block::blocks::Blocks::Skull { .. }
        | crate::server::block::blocks::Blocks::FlowerPot { .. }
        | crate::server::block::blocks::Blocks::RedstoneComparator { .. }
        | crate::server::block::blocks::Blocks::PoweredRedstoneComparator { .. }
        | crate::server::block::blocks::Blocks::RedstoneRepeater { .. }
        | crate::server::block::blocks::Blocks::PoweredRedstoneRepeater { .. }
        | crate::server::block::blocks::Blocks::Rail { .. }
        | crate::server::block::blocks::Blocks::PoweredRail { .. }
        | crate::server::block::blocks::Blocks::DetectorRail { .. }
        | crate::server::block::blocks::Blocks::DaylightSensor { .. }
        | crate::server::block::blocks::Blocks::InvertedDaylightSensor { .. }
        | crate::server::block::blocks::Blocks::Ladder { .. }
        | crate::server::block::blocks::Blocks::Trapdoor { open: true, .. }
        | crate::server::block::blocks::Blocks::IronTrapdoor { open: true, .. }
        | crate::server::block::blocks::Blocks::SpruceFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::BirchFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::JungleFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::DarkOakFenceGate { open: true, .. }
        | crate::server::block::blocks::Blocks::AcaciaFenceGate { open: true, .. } => true,
        _ => false,
    }
}



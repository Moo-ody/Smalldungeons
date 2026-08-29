use crate::net::protocol::play::clientbound::EntityVelocity;
use crate::net::protocol::play::clientbound::PositionLook;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::{ClientId, Player};
use crate::server::block::block_collision::get_block_aabb;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::aabb::AABB;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::var_int::VarInt;
use anyhow::Result;

/// Pearl entity implementation, ported from RustClear's simpler gravity/drag/AABB-overlap
/// physics (confirmed to feel better in practice than the previous raytrace+swept-AABB version).
pub struct PearlEntityImpl {
    thrower_client_id: ClientId,
    spawn_velocity: DVec3,
}

impl PearlEntityImpl {
    pub fn new(thrower_client_id: ClientId, velocity: DVec3) -> Self {
        Self {
            thrower_client_id,
            spawn_velocity: velocity,
        }
    }
}

impl EntityImpl for PearlEntityImpl {
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Inform clients of initial motion so the projectile animates
        for _player in entity.world_mut().players.values() {
            let _ = packet_buffer.write_packet(&EntityVelocity {
                entity_id: entity.id,
                velocity_x: self.spawn_velocity.x,
                velocity_y: self.spawn_velocity.y,
                velocity_z: self.spawn_velocity.z,
            });
        }
        entity.velocity = self.spawn_velocity;
    }

    fn tick(&mut self, entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
        const GRAVITY: f64 = 0.03;
        const DRAG: f64 = 0.99;

        entity.velocity.y -= GRAVITY;
        entity.velocity *= DRAG;
        entity.position += entity.velocity;

        let world = entity.world_mut();
        let aabb = AABB::from_width_height(0.25, 0.25).offset(entity.position);
        let last_aabb = AABB::from_width_height(0.25, 0.25).offset(entity.last_position);

        if check_block_collisions(world, &aabb) || check_block_collisions(world, &last_aabb) {
            if let Some(player) = world.players.get_mut(&self.thrower_client_id) {
                let land_pos = DVec3::new(
                    entity.last_position.x.floor() + 0.5,
                    entity.last_position.y.floor() + 1.0,
                    entity.last_position.z.floor() + 0.5,
                );
                player.write_packet(&PositionLook {
                    x: land_pos.x,
                    y: land_pos.y,
                    z: land_pos.z,
                    yaw: 0.0,
                    pitch: 0.0,
                    flags: 24, // keep yaw/pitch, set absolute xyz
                });
                // PositionLook only moves the client - update the server's own record of the
                // player's position to match immediately, so the per-tick view-diff notices the
                // move and sends chunks for the landing area (see etherwarp.rs for the same fix).
                player.position = land_pos;
                player.last_position = land_pos;
            }
            world.despawn_entity(entity.id);
            return;
        }

        const MAX_TICKS: u32 = 200;
        if entity.ticks_existed > MAX_TICKS {
            world.despawn_entity(entity.id);
        }
    }
}

/// Any block whose AABB (accounting for partial shapes like slabs/liquids via `get_block_aabb`)
/// overlaps the given box.
fn check_block_collisions(world: &mut crate::server::world::World, aabb: &AABB) -> bool {
    let min_bx = aabb.min.x.floor() as i32;
    let max_bx = aabb.max.x.ceil() as i32;
    let min_by = aabb.min.y.floor() as i32;
    let max_by = aabb.max.y.ceil() as i32;
    let min_bz = aabb.min.z.floor() as i32;
    let max_bz = aabb.max.z.ceil() as i32;

    for bx in min_bx..=max_bx {
        for by in min_by..=max_by {
            for bz in min_bz..=max_bz {
                let block = world.get_block_at(bx, by, bz);
                if let Some(block_aabb) = get_block_aabb(block, bx, by, bz) {
                    if aabb.intersects(&block_aabb) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

pub fn on_right_click(player: &mut Player) -> Result<()> {
    let eye_height = 1.62; // player eye height in blocks
    let eye_pos = DVec3::new(
        player.position.x,
        player.position.y + eye_height,
        player.position.z,
    );

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
    let spawn_pos = DVec3::new(
        eye_pos.x + dir.x * 0.2,
        eye_pos.y + dir.y * 0.2,
        eye_pos.z + dir.z * 0.2,
    ); // slight offset in front of player

    player.world_mut().spawn_entity(
        spawn_pos,
        EntityMetadata::new(EntityVariant::EnderPearl),
        PearlEntityImpl::new(player.client_id, velocity),
    )?;

    Ok(())
}

/// Convenience wrapper for dungeon item handlers and other call sites.
/// This matches the `throw_pearl(player)` signature used elsewhere.
pub fn throw_pearl(player: &mut Player) -> Result<()> {
    on_right_click(player)
}

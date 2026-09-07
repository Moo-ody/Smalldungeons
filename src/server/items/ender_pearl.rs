use crate::net::protocol::play::clientbound::EntityVelocity;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::{ClientId, Player};
use crate::server::block::block_collision::get_block_aabb;
use crate::server::block::blocks::Blocks;
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
        // Largest single substep this tick's movement gets divided into - well under both a
        // slab's half-block thickness and the pearl's own 0.25-wide hitbox, so a fast-moving
        // pearl (enough velocity that a whole tick's movement is several blocks) can never
        // tunnel past a thin obstacle - e.g. clipping through a slab from underneath - between
        // two consecutive collision checks the way a single start/end-of-tick check could.

        const MAX_SUBSTEP: f64 = 0.2;

        entity.velocity.y -= GRAVITY;
        entity.velocity *= DRAG;

        let travel = entity.velocity;
        let distance = (travel.x * travel.x + travel.y * travel.y + travel.z * travel.z).sqrt();
        let substeps = ((distance / MAX_SUBSTEP).ceil() as u32).max(1);
        let step = DVec3::new(travel.x / substeps as f64, travel.y / substeps as f64, travel.z / substeps as f64);

        let world = entity.world_mut();
        let mut current = entity.position;
        let mut hit = false;

        for _ in 0..substeps {
            let candidate = current + step;
            let aabb = AABB::from_width_height(0.25, 0.25).offset(candidate);
            if check_block_collisions(world, &aabb) {
                hit = true;
                break;
            }
            current = candidate;
        }
        entity.position = current;

        if hit {
            if let Some(player) = world.players.get_mut(&self.thrower_client_id) {
                let land_pos = DVec3::new(
                    current.x.floor() + 0.5,
                    current.y.floor() + 1.0,
                    current.z.floor() + 0.5,
                );
                // keep yaw/pitch, set absolute xyz - `server_teleport` handles both the
                // immediate authoritative position update (so chunks for the landing area get
                // sent right away) and rejecting stale pre-teleport position reports.
                player.server_teleport(land_pos, 0.0, 0.0, 24);
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

/// Every block type pearls fly straight through as if it were air, never landing on or being
/// stopped by one - per explicit request: every real pressure plate variant, and skulls. Scoped
/// to this file's own local `check_block_collisions` (pearl-only) rather than the shared
/// `block_collision::get_block_aabb` every other system (mob physics/pathfinding, other
/// projectiles) also uses, so this doesn't change how anything else in the game treats them.
fn pearl_ignores(block: Blocks) -> bool {
    matches!(
        block,
        Blocks::StonePressurePlate { .. }
            | Blocks::WoodenPressurePlate { .. }
            | Blocks::GoldPressurePlate { .. }
            | Blocks::IronPressurePlate { .. }
            | Blocks::Skull { .. }
    )
}

/// Any block whose AABB (accounting for partial shapes like slabs/liquids via `get_block_aabb`)
/// overlaps the given box - except whatever `pearl_ignores` always treats as air.
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
                if pearl_ignores(block) {
                    continue;
                }
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

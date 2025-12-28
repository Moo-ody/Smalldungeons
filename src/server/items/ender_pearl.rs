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

/// Pearl entity implementation with vanilla 1.8.9 physics
pub struct PearlEntityImpl {
    thrower_client_id: ClientId,
    velocity: DVec3,
}

impl PearlEntityImpl {
    pub fn new(thrower_client_id: ClientId, velocity: DVec3) -> Self {
        Self {
            thrower_client_id,
            velocity,
        }
    }
}

impl EntityImpl for PearlEntityImpl {
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Inform clients of initial motion so the projectile animates
        for _player in entity.world_mut().players.values() {
            let _ = packet_buffer.write_packet(&EntityVelocity {
                entity_id: VarInt(entity.id),
                velocity_x: (self.velocity.x * 8000.0) as i16,
                velocity_y: (self.velocity.y * 8000.0) as i16,
                velocity_z: (self.velocity.z * 8000.0) as i16,
            });
        }
        entity.velocity = self.velocity;
    }

    fn tick(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Vanilla 1.8.9 projectile physics pipeline:
        // 1. Move with current velocity
        // 2. Raytrace from old → new position
        // 3. If hit: teleport owner & remove pearl
        // 4. Else: commit position, apply drag & gravity
        
        let old_pos = entity.position;
        
        // 1) Calculate new position after full motion
        let new_pos = DVec3::new(
            entity.position.x + self.velocity.x,
            entity.position.y + self.velocity.y,
            entity.position.z + self.velocity.z,
        );

        // 2) Raytrace from oldPos to newPos
        let raytrace_result = raytrace_to_block(entity.world_mut(), old_pos, new_pos);
        
        // 3) AABB collision check
        let collision_result = check_aabb_collision(entity.world_mut(), old_pos, new_pos);
        
        // Use the earliest collision point between raytrace and AABB
        let collision = match (raytrace_result, collision_result) {
            (Some(rt), Some(cc)) => {
                let rt_dist = rt.hit_pos.distance_squared(&old_pos);
                let cc_dist = cc.hit_pos.distance_squared(&old_pos);
                if rt_dist < cc_dist {
                    Some(rt)
                } else {
                    Some(cc)
                }
            }
            (Some(rt), None) => Some(rt),
            (None, Some(cc)) => Some(cc),
            (None, None) => None,
        };

        // 3) If hit: teleport owner & remove pearl
        if let Some(collision) = collision {
            let world = entity.world_mut();
            teleport_owner_from_hit(&self.thrower_client_id, &collision, world, packet_buffer);
            world.despawn_entity(entity.id);
            return;
        }

        // 4) No hit: commit position, apply drag & gravity
        entity.position = new_pos;
        
        // Apply drag & gravity (vanilla throwable constants)
        const DRAG: f64 = 0.99;
        const GRAVITY: f64 = 0.03;
        
        self.velocity.x *= DRAG;
        self.velocity.y *= DRAG;
        self.velocity.z *= DRAG;
        self.velocity.y -= GRAVITY;
        
        entity.velocity = self.velocity;

        // Despawn after max ticks (no teleport)
        const MAX_TICKS: u32 = 200;
        if entity.ticks_existed > MAX_TICKS {
            entity.world_mut().despawn_entity(entity.id);
        }
    }
}

#[derive(Debug, Clone)]
struct CollisionResult {
    hit_pos: DVec3,
    hit_face: HitFace,
    block_pos: (i32, i32, i32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HitFace {
    Top,
    Side(DVec3), // normal vector
    Down,
}

/// Raytrace from start to end, returning first block hit
fn raytrace_to_block(world: &mut crate::server::world::World, start: DVec3, end: DVec3) -> Option<CollisionResult> {
    // Use DDA (Digital Differential Analyzer) for voxel traversal
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let dz = end.z - start.z;
    
    let step_x = if dx > 0.0 { 1 } else { -1 };
    let step_y = if dy > 0.0 { 1 } else { -1 };
    let step_z = if dz > 0.0 { 1 } else { -1 };
    
    let mut t_max_x = if dx != 0.0 {
        ((if step_x > 0 { start.x.floor() as i32 + 1 } else { start.x.floor() as i32 }) as f64 - start.x) / dx
    } else {
        f64::MAX
    };
    let mut t_max_y = if dy != 0.0 {
        ((if step_y > 0 { start.y.floor() as i32 + 1 } else { start.y.floor() as i32 }) as f64 - start.y) / dy
    } else {
        f64::MAX
    };
    let mut t_max_z = if dz != 0.0 {
        ((if step_z > 0 { (start.z.floor() as i32 + 1) } else { start.z.floor() as i32 }) as f64 - start.z) / dz
    } else {
        f64::MAX
    };
    
    let t_delta_x = if dx != 0.0 { (step_x as f64) / dx } else { f64::MAX };
    let t_delta_y = if dy != 0.0 { (step_y as f64) / dy } else { f64::MAX };
    let t_delta_z = if dz != 0.0 { (step_z as f64) / dz } else { f64::MAX };
    
    let mut x = start.x.floor() as i32;
    let mut y = start.y.floor() as i32;
    let mut z = start.z.floor() as i32;
    
    let end_x = end.x.floor() as i32;
    let end_y = end.y.floor() as i32;
    let end_z = end.z.floor() as i32;
    
    let mut last_t = 0.0;
    
    loop {
        // Check if we've passed the end
        if (x - end_x).abs() > 1 || (y - end_y).abs() > 1 || (z - end_z).abs() > 1 {
            break;
        }
        
        let block = world.get_block_at(x, y, z);
        if let Some(block_aabb) = get_block_aabb(block, x, y, z) {
            // Check if ray intersects this block's AABB
            if let Some((hit_pos, hit_face)) = ray_aabb_intersect(start, end, &block_aabb) {
                return Some(CollisionResult {
                    hit_pos,
                    hit_face,
                    block_pos: (x, y, z),
                });
            }
        }
        
        // Step to next voxel
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                last_t = t_max_x;
                t_max_x += t_delta_x;
                x += step_x;
            } else {
                last_t = t_max_z;
                t_max_z += t_delta_z;
                z += step_z;
            }
        } else {
            if t_max_y < t_max_z {
                last_t = t_max_y;
                t_max_y += t_delta_y;
                y += step_y;
            } else {
                last_t = t_max_z;
                t_max_z += t_delta_z;
                z += step_z;
            }
        }
        
        if last_t > 1.0 {
            break;
        }
    }
    
    None
}

/// Check AABB collision along the movement segment
fn check_aabb_collision(world: &mut crate::server::world::World, start: DVec3, end: DVec3) -> Option<CollisionResult> {
    // Pearl AABB is small but not zero (approximately 0.25x0.25x0.25)
    const PEARL_SIZE: f64 = 0.125; // half-size
    let pearl_aabb = AABB::new(
        DVec3::new(start.x - PEARL_SIZE, start.y - PEARL_SIZE, start.z - PEARL_SIZE),
        DVec3::new(start.x + PEARL_SIZE, start.y + PEARL_SIZE, start.z + PEARL_SIZE),
    );
    
    // Sweep AABB along the movement
    let min_x = start.x.min(end.x) - PEARL_SIZE;
    let max_x = start.x.max(end.x) + PEARL_SIZE;
    let min_y = start.y.min(end.y) - PEARL_SIZE;
    let max_y = start.y.max(end.y) + PEARL_SIZE;
    let min_z = start.z.min(end.z) - PEARL_SIZE;
    let max_z = start.z.max(end.z) + PEARL_SIZE;
    
    let min_bx = min_x.floor() as i32;
    let max_bx = max_x.ceil() as i32;
    let min_by = min_y.floor() as i32;
    let max_by = max_y.ceil() as i32;
    let min_bz = min_z.floor() as i32;
    let max_bz = max_z.ceil() as i32;
    
    let mut earliest_collision: Option<(f64, CollisionResult)> = None;
    
    for bx in min_bx..=max_bx {
        for by in min_by..=max_by {
            for bz in min_bz..=max_bz {
                let block = world.get_block_at(bx, by, bz);
                if let Some(block_aabb) = get_block_aabb(block, bx, by, bz) {
                    // Check swept AABB collision
                    if let Some((t, hit_pos, hit_face)) = swept_aabb_collision(
                        pearl_aabb.clone(),
                        end - start,
                        &block_aabb,
                    ) {
                        if t >= 0.0 && t <= 1.0 {
                            let should_update = match &earliest_collision {
                                None => true,
                                Some((existing_t, _)) => t < *existing_t,
                            };
                            if should_update {
                                earliest_collision = Some((t, CollisionResult {
                                    hit_pos,
                                    hit_face,
                                    block_pos: (bx, by, bz),
                                }));
                            }
                        }
                    }
                }
            }
        }
    }
    
    earliest_collision.map(|(_, result)| result)
}

/// Teleport owner from block hit (vanilla 1.8.9 style)
/// Simplified: teleport to pearl position minus eye height, then unstuck
fn teleport_owner_from_hit(
    owner_id: &ClientId,
    hit: &CollisionResult,
    world: &mut crate::server::world::World,
    _packet_buffer: &mut PacketBuffer,
) {
    // Teleport based on pearl position, not block coords
    // pearl position = approximate body/eye area
    let mut feet = hit.hit_pos;
    
    // Shift to feet (eye height)
    const EYE_HEIGHT: f64 = 1.62;
    feet.y -= EYE_HEIGHT;
    
    // Small safety: push up if we're inside a block (capped at 2 steps)
    feet = resolve_upward(feet, world);
    
    // Teleport player
    if let Some(player) = world.players.get_mut(owner_id) {
        player.write_packet(&PositionLook {
            x: feet.x,
            y: feet.y,
            z: feet.z,
            yaw: 0.0,
            pitch: 0.0,
            flags: 24, // keep yaw/pitch, set absolute xyz
        });
    }
}

/// Upward collision resolution (the "climb" mechanic)
/// This is the only "unstuck" logic - runs for every teleport
/// feet.y is player feet position
/// Capped at 2 iterations to prevent +4-6 block rockets
fn resolve_upward(mut feet: DVec3, world: &mut crate::server::world::World) -> DVec3 {
    const MAX_RESOLVE_ATTEMPTS: usize = 2;
    
    for _ in 0..MAX_RESOLVE_ATTEMPTS {
        let aabb = player_aabb_at(feet);
        
        if !collides_with_any_solid(world, &aabb) {
            break; // No collision, we're done
        }
        
        // Nudge upward
        feet.y += 1.0;
    }
    
    feet
}

/// Get player AABB at feet position (vanilla model)
/// feet.y is the player's feet position
fn player_aabb_at(feet: DVec3) -> AABB {
    const PLAYER_WIDTH: f64 = 0.3; // half-width (total width = 0.6)
    const PLAYER_HEIGHT: f64 = 1.8;
    
    AABB::new(
        DVec3::new(feet.x - PLAYER_WIDTH, feet.y, feet.z - PLAYER_WIDTH),
        DVec3::new(feet.x + PLAYER_WIDTH, feet.y + PLAYER_HEIGHT, feet.z + PLAYER_WIDTH),
    )
}

/// Check if player AABB collides with any solid blocks
/// Uses half-open intervals: touching does NOT count as collision
fn collides_with_any_solid(world: &mut crate::server::world::World, aabb: &AABB) -> bool {
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
                    if aabb_collides_half_open(aabb, &block_aabb) {
                        return true;
                    }
                }
            }
        }
    }
    
    false
}

/// Check if two AABBs collide using half-open intervals (vanilla-style)
/// Touching (max_y == other.min_y) does NOT count as collision
fn aabb_collides_half_open(aabb1: &AABB, aabb2: &AABB) -> bool {
    // Check if completely separated (using half-open intervals)
    if aabb1.max.x <= aabb2.min.x || aabb1.min.x >= aabb2.max.x {
        return false;
    }
    if aabb1.max.y <= aabb2.min.y || aabb1.min.y >= aabb2.max.y {
        return false;
    }
    if aabb1.max.z <= aabb2.min.z || aabb1.min.z >= aabb2.max.z {
        return false;
    }
    // If none of the separation checks passed, they overlap
    true
}


/// Ray-AABB intersection test
fn ray_aabb_intersect(start: DVec3, end: DVec3, aabb: &AABB) -> Option<(DVec3, HitFace)> {
    let dir = end - start;
    let inv_dir = DVec3::new(
        if dir.x != 0.0 { 1.0 / dir.x } else { f64::MAX },
        if dir.y != 0.0 { 1.0 / dir.y } else { f64::MAX },
        if dir.z != 0.0 { 1.0 / dir.z } else { f64::MAX },
    );
    
    let t1 = (aabb.min.x - start.x) * inv_dir.x;
    let t2 = (aabb.max.x - start.x) * inv_dir.x;
    let t3 = (aabb.min.y - start.y) * inv_dir.y;
    let t4 = (aabb.max.y - start.y) * inv_dir.y;
    let t5 = (aabb.min.z - start.z) * inv_dir.z;
    let t6 = (aabb.max.z - start.z) * inv_dir.z;
    
    let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
    let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));
    
    if tmax < 0.0 || tmin > tmax || tmin > 1.0 {
        return None;
    }
    
    let t = if tmin < 0.0 { tmax } else { tmin };
    let hit_pos = DVec3::new(
        start.x + dir.x * t,
        start.y + dir.y * t,
        start.z + dir.z * t,
    );
    
    // Determine hit face by checking which plane was hit
    let epsilon = 1e-6;
    let hit_face = if (hit_pos.y - aabb.max.y).abs() < epsilon {
        HitFace::Top
    } else if (hit_pos.y - aabb.min.y).abs() < epsilon {
        HitFace::Down
    } else {
        // Side face - determine normal from which face was hit
        // Check which X or Z face was hit (not Y)
        let normal = if (hit_pos.x - aabb.max.x).abs() < epsilon {
            // East face
            DVec3::new(1.0, 0.0, 0.0)
        } else if (hit_pos.x - aabb.min.x).abs() < epsilon {
            // West face
            DVec3::new(-1.0, 0.0, 0.0)
        } else if (hit_pos.z - aabb.max.z).abs() < epsilon {
            // South face
            DVec3::new(0.0, 0.0, 1.0)
        } else if (hit_pos.z - aabb.min.z).abs() < epsilon {
            // North face
            DVec3::new(0.0, 0.0, -1.0)
        } else {
            // Fallback: compute from hit position to center
            let center = DVec3::new(
                (aabb.min.x + aabb.max.x) * 0.5,
                (aabb.min.y + aabb.max.y) * 0.5,
                (aabb.min.z + aabb.max.z) * 0.5,
            );
            let mut normal = hit_pos - center;
            normal.y = 0.0; // Only X/Z components
            normal.normalize()
        };
        HitFace::Side(normal)
    };
    
    Some((hit_pos, hit_face))
}

/// Swept AABB collision test
fn swept_aabb_collision(
    moving_aabb: AABB,
    movement: DVec3,
    static_aabb: &AABB,
) -> Option<(f64, DVec3, HitFace)> {
    // Simplified swept AABB - check if moving AABB at t=0 and t=1 intersects
    // For more accuracy, we'd need proper swept AABB, but this approximation works
    if moving_aabb.intersects(static_aabb) {
        // Already intersecting at start
        let center = DVec3::new(
            (static_aabb.min.x + static_aabb.max.x) * 0.5,
            (static_aabb.min.y + static_aabb.max.y) * 0.5,
            (static_aabb.min.z + static_aabb.max.z) * 0.5,
        );
        let moving_center = DVec3::new(
            (moving_aabb.min.x + moving_aabb.max.x) * 0.5,
            (moving_aabb.min.y + moving_aabb.max.y) * 0.5,
            (moving_aabb.min.z + moving_aabb.max.z) * 0.5,
        );
        // Determine hit face from which face of the block was hit
        let diff = moving_center - center;
        let hit_face = if diff.y.abs() > diff.x.abs() && diff.y.abs() > diff.z.abs() {
            if diff.y > 0.0 {
                HitFace::Top
            } else {
                HitFace::Down
            }
        } else {
            // Side face - extract only X/Z components
            let mut normal = DVec3::new(diff.x, 0.0, diff.z);
            normal = normal.normalize();
            HitFace::Side(normal)
        };
        
        return Some((0.0, moving_center, hit_face));
    }
    
    // Check end position
    let end_aabb = AABB::new(
        moving_aabb.min + movement,
        moving_aabb.max + movement,
    );
    
    if end_aabb.intersects(static_aabb) {
        let center = DVec3::new(
            (static_aabb.min.x + static_aabb.max.x) * 0.5,
            (static_aabb.min.y + static_aabb.max.y) * 0.5,
            (static_aabb.min.z + static_aabb.max.z) * 0.5,
        );
        let end_center = DVec3::new(
            (end_aabb.min.x + end_aabb.max.x) * 0.5,
            (end_aabb.min.y + end_aabb.max.y) * 0.5,
            (end_aabb.min.z + end_aabb.max.z) * 0.5,
        );
        // Determine hit face from which face of the block was hit
        let diff = end_center - center;
        let hit_face = if diff.y.abs() > diff.x.abs() && diff.y.abs() > diff.z.abs() {
            if diff.y > 0.0 {
                HitFace::Top
            } else {
                HitFace::Down
            }
        } else {
            // Side face - extract only X/Z components
            let mut normal = DVec3::new(diff.x, 0.0, diff.z);
            normal = normal.normalize();
            HitFace::Side(normal)
        };
        
        return Some((1.0, end_center, hit_face));
    }
    
    None
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

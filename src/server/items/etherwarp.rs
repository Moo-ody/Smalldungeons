use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::SendPacket;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::Entity;
use crate::server::player::Player;
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;
use std::f64::consts::PI;
use tokio::sync::mpsc::UnboundedSender;

pub fn handle_etherwarp(
    player: &Player,
    network_tx: &UnboundedSender<NetworkMessage>,
    world: &World,
    entity: &Entity
) -> anyhow::Result<()> {
    let mut start_pos = entity.pos.clone();
    start_pos.y += 1.54; // assume always sneaking

    let end_pos = {
        let yaw = entity.yaw as f64;
        let pitch = entity.pitch as f64;
        let rad_yaw = -yaw.to_radians() - PI;
        let rad_pitch = -pitch.to_radians();

        let f2 = -rad_pitch.cos();
        let mut pos = Vec3f {
            x: rad_yaw.sin() * f2,
            y: rad_pitch.sin(),
            z: rad_yaw.cos() * f2,
        }.normalize();

        pos.x *= 61.0;
        pos.y *= 61.0;
        pos.z *= 61.0;

        pos + start_pos
    };

    if let EtherResult::Valid(x, y, z) = traverse_voxels(world, start_pos, end_pos) {
        PositionLook {
            x: x as f64 + 0.5,
            y: y as f64 + 1.05,
            z: z as f64 + 0.5,
            yaw: 0.0,
            pitch: 0.0,
            // flags make x y z absolute, and yaw/pitch relative,
            // since yaw and pitch is 0, it doesn't rotate the player
            flags: 24,
        }.send_packet(player.client_id, network_tx)?;
    }
    Ok(())
}

enum EtherResult {
    Valid(i32, i32, i32),
    Failed,
}

/// gets the position

fn get_ether_position(
    world: &World,
    entity: &Entity,
) -> Option<BlockPos> {
    let start_pos = {
        let mut pos = entity.pos.clone();
        pos.y += 1.54; // assume always sneaking
        pos
    };
    let end_pos = {
        let mut pos = get_look(entity.yaw, entity.pitch).normalize();
        pos.x *= 61.0;
        pos.y *= 61.0;
        pos.z *= 61.0;
        pos + start_pos
    };
    if let EtherResult::Valid(x, y, z) = traverse_voxels(world, start_pos, end_pos) {
        return Some(BlockPos { x, y, z })
    }
    None
}

fn get_look(yaw: f32, pitch: f32) -> Vec3f {
    let f2 = -(-pitch * 0.017453292).cos() as f64;
    Vec3f {
        x: (-yaw as f64 * 0.017453292 - PI).sin() * f2,
        y: (-pitch as f64 * 0.017453292).sin(),
        z: (-yaw as f64 * 0.017453292 - PI).cos() * f2
    }
}

fn traverse_voxels(world: &World, start: Vec3f, end: Vec3f) -> EtherResult {
    let (x0, y0, z0) = (start.x, start.y, start.z);
    let (x1, y1, z1) = (end.x, end.y, end.z);

    let (mut x, mut y, mut z) = (start.x.floor() as i32, start.y.floor() as i32, start.z.floor() as i32);
    let (end_x, end_y, end_z) = (end.x.floor() as i32, end.y.floor() as i32, end.z.floor() as i32);

    let dir_x = x1 - x0;
    let dir_y = y1 - y0;
    let dir_z = z1 - z0;

    let step_x = dir_x.signum() as i32;
    let step_y = dir_y.signum() as i32;
    let step_z = dir_z.signum() as i32;

    let inv_dir_x = if dir_x != 0.0 { 1.0 / dir_x } else { f64::MAX };
    let inv_dir_y = if dir_y != 0.0 { 1.0 / dir_y } else { f64::MAX };
    let inv_dir_z = if dir_z != 0.0 { 1.0 / dir_z } else { f64::MAX };

    let t_delta_x = (inv_dir_x * step_x as f64).abs();
    let t_delta_y = (inv_dir_y * step_y as f64).abs();
    let t_delta_z = (inv_dir_z * step_z as f64).abs();

    // t_max initialization follows the "next voxel boundary" logic
    let mut t_max_x = ((x as f64 + if step_x > 0 { 1.0 } else { 0.0 } - x0) * inv_dir_x).abs();
    let mut t_max_y = ((y as f64 + if step_y > 0 { 1.0 } else { 0.0 } - y0) * inv_dir_y).abs();
    let mut t_max_z = ((z as f64 + if step_z > 0 { 1.0 } else { 0.0 } - z0) * inv_dir_z).abs();

    for _ in 0..1000 {
        // Check block at current voxel coordinates
        let current_block = world.get_block_at(x, y, z);

        if current_block != Blocks::Air {
            // Check if above blocks are air as in Kotlin code
            if world.get_block_at(x, y + 1, z) == Blocks::Air && world.get_block_at(x, y + 2, z) == Blocks::Air {
                return EtherResult::Valid(x, y, z);
            } else {
                return EtherResult::Failed;
            }
        }

        if x == end_x && y == end_y && z == end_z {
            return EtherResult::Failed;
        }

        if t_max_x <= t_max_y && t_max_x <= t_max_z {
            t_max_x += t_delta_x;
            x += step_x;
        } else if t_max_y <= t_max_z {
            t_max_y += t_delta_y;
            y += step_y;
        } else {
            t_max_z += t_delta_z;
            z += step_z;
        }
    }

    EtherResult::Failed
}
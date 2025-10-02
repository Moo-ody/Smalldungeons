use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::protocol::play::clientbound::{Particles, PositionLook, SoundEffect};
use crate::server::block::blocks::Blocks;
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use crate::utils::bitset::BitSet;
use std::f64::consts::PI;
use tokio::sync::mpsc::UnboundedSender;

const VALID_ETHER_WARP_BLOCK_IDS: BitSet<3> = BitSet::new(
    &[
        0, 6, 9, 11, 30, 31, 32, 36, 37, 38, 39, 40, 50, 51, 55, 59, 65, 66, 69, 76, 77, 78,
        93, 94, 104, 105, 106, 111, 115, 131, 132, 140, 141, 142, 143, 144, 149, 150, 157, 171, 175
    ]
);

enum EtherResult {
    Valid(i32, i32, i32),
    Failed,
}

pub fn handle_ether_warp(
    player: &mut Player,
    world: &World,
) -> anyhow::Result<()> {
    let mut start_pos = player.position.clone();
    start_pos.y += 1.54; // assume always sneaking

    let end_pos = {
        let yaw = player.yaw as f64;
        let pitch = player.pitch as f64;
        let rad_yaw = -yaw.to_radians() - PI;
        let rad_pitch = -pitch.to_radians();

        let f2 = -rad_pitch.cos();

        let mut pos = DVec3 {
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
        player.write_packet(&Particles {
            particle_id: 17,
            long_distance: true,
            x: player.position.x as f32,
            y: player.position.y as f32,
            z: player.position.z as f32,
            offset_x: 0.25,
            offset_y: 1.0,
            offset_z: 0.25,
            speed: 0.0,
            count: 25,
        });
        player.write_packet(&PositionLook {
            x: x as f64 + 0.5,
            y: y as f64 + 1.05,
            z: z as f64 + 0.5,
            yaw: 0.0,
            pitch: 0.0,
            // these flags make xyz absolute meaning they set directly
            // while keeping yaw and pitch relative (meaning it is added to players yaw)
            // since yaw and pitch provided is 0, it doesn't rotate the player causing head snapping
            flags: 24,
        });
        player.write_packet(&SoundEffect {
            sound: "mob.enderdragon.hit",
            volume: 1.0,
            pitch: 0.53968257,
            pos_x: x as f64 + 0.5,
            pos_y: y as f64 + 1.05,
            pos_z: z as f64 + 0.5,
        });
    }
    Ok(())
}

fn traverse_voxels(world: &World, start: DVec3, end: DVec3) -> EtherResult {
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

        if !VALID_ETHER_WARP_BLOCK_IDS.contains((current_block.get_block_state_id() >> 4) as usize) {
            let block_up1 = world.get_block_at(x, y + 1, z).get_block_state_id() >> 4;
            let block_up2 = world.get_block_at(x, y + 2, z).get_block_state_id() >> 4;

            return if VALID_ETHER_WARP_BLOCK_IDS.contains(block_up1 as usize) && VALID_ETHER_WARP_BLOCK_IDS.contains(block_up2 as usize) {
                EtherResult::Valid(x, y, z)
            } else {
                EtherResult::Failed
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

const MAX_DISTANCE: f64 = 12.0;

pub fn handle_teleport(
    player: &mut Player,
    _network_tx: &UnboundedSender<NetworkThreadMessage>,
) -> anyhow::Result<()> {
    // Start from eye position
    let mut start = player.position;
    start.y += 1.62;

    // Direction from yaw/pitch (1.8)
    let yaw = player.yaw as f64;
    let pitch = player.pitch as f64;
    let rad_yaw = -yaw.to_radians() - PI;
    let rad_pitch = -pitch.to_radians();
    let f2 = -rad_pitch.cos();
    let dir = DVec3 {
        x: rad_yaw.sin() * f2,
        y: rad_pitch.sin(),
        z: rad_yaw.cos() * f2,
    }.normalize();

    // Swept ray up to MAX_DISTANCE, track last two-high passable cell
    let step_len = 0.25f64;
    let steps = (MAX_DISTANCE / step_len).ceil() as i32;
    let step = DVec3::new(dir.x * step_len, dir.y * step_len, dir.z * step_len);
    let mut last_safe_block: Option<(i32, i32, i32)> = None;
    let mut current = start;

    for _ in 0..steps {
        current = DVec3::new(current.x + step.x, current.y + step.y, current.z + step.z);
        let bx = current.x.floor() as i32;
        let by = current.y.floor() as i32;
        let bz = current.z.floor() as i32;

        // We want to stand in this cell: require feet and head passable
        if is_passable_for_transmission(block_at(player, bx, by, bz))
            && is_passable_for_transmission(block_at(player, bx, by + 1, bz)) {
            last_safe_block = Some((bx, by, bz));
            continue;
        } else {
            break; // hit a solid; stop in front
        }
    }

    if let Some((bx, by, bz)) = last_safe_block {
        // Final destination at center of the last safe block
        let dest_x = bx as f64 + 0.5;
        let dest_y = by as f64; // feet at block base; client packet uses absolute feet
        let dest_z = bz as f64 + 0.5;

        player.write_packet(&PositionLook {
            x: dest_x,
            y: dest_y,
            z: dest_z,
            yaw: 0.0,
            pitch: 0.0,
            flags: 24,
        });
    }

    Ok(())
}

#[inline]
fn block_at(player: &mut Player, x: i32, y: i32, z: i32) -> Blocks {
    player.server_mut().world.get_block_at(x, y, z)
}

#[inline]
fn is_passable_for_transmission(block: Blocks) -> bool {
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

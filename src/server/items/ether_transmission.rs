use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::protocol::play::clientbound::PositionLook;
use crate::server::block::blocks::Blocks;
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use std::f64::consts::PI;
use tokio::sync::mpsc::UnboundedSender;



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

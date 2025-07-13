use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::packet::SendPacket;
use crate::server::entity::entity::Entity;
use crate::server::player::Player;
use crate::server::utils::vec3d::DVec3;
use crate::server::world::World;
use std::f64::consts::PI;
use tokio::sync::mpsc::UnboundedSender;

pub fn handle_teleport(
    player: &Player,
    network_tx: &UnboundedSender<NetworkThreadMessage>,
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
        let mut pos = DVec3 {
            x: rad_yaw.sin() * f2,
            y: rad_pitch.sin(),
            z: rad_yaw.cos() * f2,
        }.normalize();

        pos.x *= 10.0;
        pos.y *= 10.0;
        pos.z *= 10.0;

        pos + start_pos
    };

    PositionLook {
        x: end_pos.x as f64 + 0.5,
        y: end_pos.y as f64 + 1.05,
        z: end_pos.z as f64 + 0.5,
        yaw: 0.0,
        pitch: 0.0,
        // flags make x y z absolute, and yaw/pitch relative,
        // since yaw and pitch is 0, it doesn't rotate the player
        flags: 24,
    }.send_packet(player.client_id, network_tx)?;

    Ok(())
}

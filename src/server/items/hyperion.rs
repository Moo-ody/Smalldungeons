use crate::net::packets::client_bound::sound_effect::SoundEffect;
use crate::net::packets::packet::SendPacket;
use crate::server::player::player::Player;

pub fn on_right_click(player: &mut Player) -> anyhow::Result<()> {
    // Use the exact same teleport logic as ether transmission, but with 10 blocks
    let server = &player.server_mut();
    let teleport_result = crate::server::items::ether_transmission::handle_teleport(player, &server.network_tx, 10.0);
    
    // Only play sounds if teleport was successful
    if teleport_result.is_ok() {
        // Always play endermen portal sound on right-click
        let _ = SoundEffect {
            sounds: crate::server::utils::sounds::Sounds::EndermenPortal,
            volume: 1.0,
            pitch: 1.0,
            x: player.position.x,
            y: player.position.y,
            z: player.position.z,
        }.send_packet(player.client_id, &player.network_tx);

        // Always play random.explode sound
        let _ = SoundEffect {
            sounds: crate::server::utils::sounds::Sounds::RandomExplode,
            volume: 1.0,
            pitch: 1.0,
            x: player.position.x,
            y: player.position.y,
            z: player.position.z,
        }.send_packet(player.client_id, &player.network_tx);
    }

    // TODO: AoE damage + particles/sounds at destination

    Ok(())
}

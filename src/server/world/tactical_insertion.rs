use crate::net::packets::client_bound::position_look::PositionLook;
use crate::net::packets::client_bound::sound_effect::SoundEffect;
use crate::net::packets::packet::SendPacket;
use crate::server::player::player::ClientId;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;

#[derive(Debug, Clone, Copy)]
pub struct TacticalInsertionMarker {
    pub client_id: ClientId,
    pub return_tick: u64,
    pub origin: DVec3,
    pub damage_echo_window_ticks: u64,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ScheduledSound {
    pub due_tick: u64,
    pub sound: Sounds,
    pub volume: f32,
    pub pitch: f32,
}

pub fn process(world: &mut World) -> anyhow::Result<()> {
    if world.tactical_insertions.is_empty() { return Ok(()); }

    let now = world.tick_count;
    let mut remaining: Vec<(TacticalInsertionMarker, Vec<ScheduledSound>)> =
        Vec::with_capacity(world.tactical_insertions.len());

    for (mut marker, mut sounds) in world.tactical_insertions.drain(..) {
        if let Some(player) = world.players.get(&marker.client_id) {
            // Make sounds follow the player by using their current position
            let (x, y, z) = (player.position.x, player.position.y, player.position.z);
            let mut future: Vec<ScheduledSound> = Vec::new();
            for s in sounds.drain(..) {
                if s.due_tick <= now {
                    let _ = SoundEffect { sounds: s.sound, volume: s.volume, pitch: s.pitch, x, y, z }
                        .send_packet(player.client_id, &player.network_tx);
                } else {
                    future.push(s);
                }
            }
            sounds = future;
        }

        if marker.return_tick <= now {
            if let Some(player) = world.players.get(&marker.client_id) {
                let _ = PositionLook {
                    x: marker.origin.x,
                    y: marker.origin.y,
                    z: marker.origin.z,
                    yaw: marker.yaw,
                    pitch: marker.pitch,
                    // 0 => absolute yaw/pitch so we face the original direction
                    flags: 0,
                }.send_packet(player.client_id, &player.network_tx);
            }
            marker.return_tick = u64::MAX; // prevent repeat
        }

        if !sounds.is_empty() {
            remaining.push((marker, sounds));
        }
    }

    world.tactical_insertions = remaining;
    Ok(())
}



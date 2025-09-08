use crate::net::protocol::play::clientbound::{PositionLook, SoundEffect};
use crate::server::player::player::ClientId;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;

/// Tactical insertion marker for teleporting back after delay
#[derive(Debug, Clone, Copy)]
pub struct TacticalInsertionMarker {
    pub client_id: ClientId,
    pub return_tick: u64,
    pub origin: DVec3,
    pub damage_echo_window_ticks: u64,
    pub yaw: f32,
    pub pitch: f32,
}

/// Scheduled sound to play at specific tick
#[derive(Debug, Clone, Copy)]
pub struct ScheduledSound {
    pub due_tick: u64,
    pub sound: Sounds,
    pub volume: f32,
    pub pitch: f32,
}

/// Process scheduled tactical insertions and return teleports
pub fn process(world: &mut World) -> anyhow::Result<()> {
    if world.tactical_insertions.is_empty() {
        return Ok(());
    }
    
    // Debug: Print current state
    println!("Processing {} tactical insertions at tick {}", world.tactical_insertions.len(), world.tick_count);
    
    // Drain due markers
    let now = world.tick_count;
    let mut remaining: Vec<(TacticalInsertionMarker, Vec<ScheduledSound>)> = Vec::with_capacity(world.tactical_insertions.len());
    
    for (mut marker, mut sounds) in world.tactical_insertions.drain(..) {
        println!("  Processing marker for player {}: return_tick={}, now={}", marker.client_id, marker.return_tick, now);
        // Emit any due sounds first
        if let Some(player) = world.players.get_mut(&marker.client_id) {
            // Make sounds follow the player by using their current position
            let (x, y, z) = (player.position.x, player.position.y, player.position.z);
            let mut future: Vec<ScheduledSound> = Vec::new();
            
            for s in sounds.drain(..) {
                if s.due_tick <= now {
                    // Send sound effect packet
                    let sound_packet = SoundEffect {
                        sound: s.sound.id(),
                        pos_x: x,
                        pos_y: y,
                        pos_z: z,
                        volume: s.volume,
                        pitch: s.pitch,
                    };
                    // Use player's write_packet method
                    player.write_packet(&sound_packet);
                } else {
                    future.push(s);
                }
            }
            sounds = future;
        }
        
        // Handle return teleport once
        if marker.return_tick <= now {
            if let Some(player) = world.players.get_mut(&marker.client_id) {
                // Send position look packet to teleport player back
                let pos_packet = PositionLook {
                    x: marker.origin.x,
                    y: marker.origin.y,
                    z: marker.origin.z,
                    yaw: marker.yaw,
                    pitch: marker.pitch,
                    flags: 0, // Try 0 first, might be absolute positioning
                };
                // Use player's write_packet method
                player.write_packet(&pos_packet);
                
                println!("  Teleported player {} back to origin", marker.client_id);
            }
            // Do not re-schedule after return
        } else {
            // Not due yet: keep scheduling regardless of pending sounds
            remaining.push((marker, sounds));
        }
    }
    
    world.tactical_insertions = remaining;
    Ok(())
}

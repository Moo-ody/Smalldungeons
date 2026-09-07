use crate::net::protocol::play::clientbound::SoundEffect;
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
    /// `true` only for a mushroom secret's own round trip (`block_interact_action.rs`'s
    /// `MushroomBottom` handler) - `false` for the real Tactical Insertion item, which reuses
    /// this exact same marker/queue for its own unrelated return trip. Gates the mushroom-only
    /// Nausea+portal-effect clearing on automatic timeout below, so a real Tactical Insertion
    /// return never gets mushroom effects applied to it just because it shares this struct.
    pub is_mushroom_secret: bool,
}

/// Scheduled sound to play at specific tick
#[derive(Debug, Clone, Copy)]
pub struct ScheduledSound {
    pub due_tick: u64,
    pub sound: Sounds,
    pub volume: f32,
    pub pitch: f32,
}

/// Scheduled sound at a fixed position (not following a player)
#[derive(Debug, Clone, Copy)]
pub struct ScheduledFixedSound {
    pub due_tick: u64,
    pub sound: Sounds,
    pub volume: f32,
    pub pitch: f32,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
}

/// Process scheduled tactical insertions and return teleports
pub fn process(world: &mut World) -> anyhow::Result<()> {
    if world.tactical_insertions.is_empty() {
        return Ok(());
    }
    
    // Debug: Print current state
    
    // Drain due markers
    let now = world.tick_count;
    let mut remaining: Vec<(TacticalInsertionMarker, Vec<ScheduledSound>)> = Vec::with_capacity(world.tactical_insertions.len());
    
    for (mut marker, mut sounds) in world.tactical_insertions.drain(..) {
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
                // `server_teleport` (not a raw `PositionLook` write) - every other server-
                // initiated teleport in this codebase goes through it specifically because it
                // updates the server's own authoritative `position` synchronously and arms
                // `pending_teleport` so a stale client report can't roll it back (see its own doc
                // comment). Writing the packet directly here left `player.position` stuck at
                // wherever the teleport-out landed until the client's next ordinary movement
                // packet happened to correct it - a real desync window, not present anywhere else
                // a teleport fires in this project.
                player.server_teleport(marker.origin, marker.yaw, marker.pitch, 0);
                if marker.is_mushroom_secret {
                    crate::dungeon::room::mushroom::clear_mushroom_up_effects(player, marker.origin);
                }
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

//! The Terminator: real item lore (`Item::Terminator` in `mod.rs`) confirms the mechanically
//! relevant stats - "Shoots 3 arrows at once", "Shot Cooldown: 0.5s", and the Shortbow reforge
//! line "Instantly shoots!" (no bow-draw/charge delay - fires the instant you click). Its other
//! stats (Damage, Crit Chance, the Precise headshot bonus) and its Salvation ability (left-click,
//! after landing 3 hits) have nothing to plug into - there's no player-vs-mob damage or
//! hit-tracking system anywhere in this codebase yet, matching how every other damage-number
//! stat on every other item here is left unimplemented rather than guessed at.
//!
//! Spread is confirmed directly from the Hypixel SkyBlock Wiki (cross-checked against two
//! independent fetches): "fires 3 arrows per shot, 1 straight and 2 spread at 5°" - "a tighter
//! spread than Runaan's Bow" - with "spread greatest when aiming horizontally and decreasing as
//! the player looks up or down." That last part isn't a separate mechanic to implement - it's
//! just what a fixed *yaw* offset (not a camera-relative rotation) naturally does as pitch
//! approaches vertical (two points at the same "latitude" on a sphere converge as that latitude
//! nears a pole), which `look_vector`'s plain yaw offset below already reproduces for free. No
//! wiki source mentions any *pitch* tilt between the 3 arrows (unlike `DungeonSim`'s own
//! `Terminator.java`, whose extra per-arrow pitch nudges and per-arrow spawn-point offset aren't
//! corroborated by this - both dropped in favor of the simpler, wiki-confirmed shape: one shared
//! origin, pure ±5° yaw, same pitch as the player's own aim for all 3).
//!
//! Also keeps that reference's one other confirmed-real detail: firing on a left-click swing,
//! not just right-click (`onSwing`/`onPlace` both call the same `useTerminator`) - matched here
//! via `use_terminator` being called from both `Item::on_right_click` and `ArmSwing`'s packet
//! handler (`packet_handling.rs`).
//!
//! Reuses `ai::projectile::fire_projectile` for the actual arrows - a real flying, gravity-
//! affected `Arrow` that arcs and despawns on hitting a block or a player, identical physics to
//! a mob-fired one (real vanilla has one `EntityArrow` class regardless of who fired it) - a
//! real vanilla-feeling shot, not a flat straight-line laser, per explicit request.

use crate::net::protocol::play::clientbound::SoundEffect;
use crate::server::entity::dungeon_mobs::ai::projectile::{fire_projectile, ImpactEffect};
use crate::server::entity::entity_metadata::EntityVariant;
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;

/// "Shot Cooldown: 0.5s" @ 20 TPS.
const SHOT_COOLDOWN_TICKS: u64 = 10;
/// Real vanilla `EntityArrow`'s own fully-charged initial speed (3.0 blocks/tick @ 20 TPS) - not
/// a made-up "fast and snappy" value. With gravity back on, matching the client's own real
/// physics reference speed (not just the gravity/drag constants) matters for the same reason -
/// any mismatch is something the per-tick server correction has to visibly fight.
const ARROW_SPEED_BPS: f64 = 60.0;
/// How far along the final aim direction the synthetic "target point" sits - `fire_projectile`
/// derives its launch direction from `(target_pos - from).normalize()`, so this just needs to be
/// far enough that the direction is accurate; it's never actually reached (the arrow despawns on
/// its own block/player hit long before 100 blocks).
const AIM_DISTANCE: f64 = 100.0;

/// "1 straight and 2 spread at 5°" (Hypixel Wiki, confirmed - see the module doc comment). All 3
/// leave from the same point (the player's eye position) at the player's own pitch - only yaw
/// differs.
const SHOT_YAW_OFFSETS_DEG: [f32; 3] = [0.0, -5.0, 5.0];

/// Standard yaw/pitch (degrees) -> normalized look-direction conversion - same formula
/// `Player::rotation_vec`/`shoot_bonzo_projectile` already use, just parameterized so it can be
/// evaluated at the small per-arrow yaw/pitch offsets above instead of only the player's own
/// exact aim.
fn look_vector(yaw_deg: f32, pitch_deg: f32) -> DVec3 {
    let yaw_rad = (yaw_deg as f64).to_radians();
    let pitch_rad = (pitch_deg as f64).to_radians();
    DVec3::new(
        -pitch_rad.cos() * yaw_rad.sin(),
        -pitch_rad.sin(),
        pitch_rad.cos() * yaw_rad.cos(),
    )
}

/// Called from both a right-click (`Item::on_right_click`) and a left-click swing (`ArmSwing`'s
/// packet handler) - see the module doc comment for why both fire it.
pub fn use_terminator(player: &mut Player) {
    let current_tick = player.world_mut().tick_count;
    if current_tick.saturating_sub(player.terminator_last_shot_tick) < SHOT_COOLDOWN_TICKS {
        return; // on cooldown - silently ignore, same convention as Bonzo/Jerry
    }
    player.terminator_last_shot_tick = current_tick;

    let origin = player.player_eye_position();
    let yaw = player.yaw;
    let pitch = player.pitch;

    player.write_packet(&SoundEffect {
        sound: Sounds::Bow.id(),
        pos_x: origin.x,
        pos_y: origin.y,
        pos_z: origin.z,
        volume: 1.0,
        pitch: 1.0,
    });

    let shooter_id = player.client_id;
    let world = player.world_mut();

    for yaw_offset in SHOT_YAW_OFFSETS_DEG {
        let direction = look_vector(yaw + yaw_offset, pitch);
        let aim_point = DVec3::new(origin.x + direction.x * AIM_DISTANCE, origin.y + direction.y * AIM_DISTANCE, origin.z + direction.z * AIM_DISTANCE);

        // `Some(shooter_id)` is what actually lets this arrow hit puzzle blocks (Creeper Beams'
        // sea lanterns, etc.) the same way clicking them directly would - see
        // `MobProjectileImpl`'s `on_block_hit` doc comment. Gravity on - a real vanilla arrow
        // (see `ai::projectile`'s `GRAVITY_PER_TICK`/`DRAG_PER_TICK`), per explicit request: it
        // should feel like an actual shot bow arcing under gravity, not a flat straight-line
        // laser.
        let _ = fire_projectile(world, origin, aim_point, ARROW_SPEED_BPS, EntityVariant::Arrow, true, ImpactEffect::Despawn, Some(shooter_id));
    }
}

pub fn on_right_click(player: &mut Player) -> anyhow::Result<()> {
    use_terminator(player);
    Ok(())
}

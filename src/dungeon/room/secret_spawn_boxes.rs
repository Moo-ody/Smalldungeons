//! Per-secret overrides for `DungeonSecret`'s spawn/reveal bounding box.
//!
//! `secrets_loader.rs` gives every secret one fixed radius per *category* (8.0 - a 16x16x16
//! box - for chests/essence/bats/items, 4.0 - 8x8x8 - for the "secret" chest/essence variants):
//! the box a player has to step into before that secret's chest/item/bat actually spawns into
//! the world (see `DungeonSecret::new`'s `spawn_radius` and `secrets.rs::tick`, which checks the
//! player's collision box against it). This module is where one specific secret - identified by
//! its room name and its own `id` from `secrets.json` - gets a different box instead of changing
//! that category's default for every secret of that kind.

/// `(room name, secret id, spawn radius)` - the radius is a half-width, so the actual box is
/// `2 * radius` on a side (a radius of `18.0` reaches a full 18 blocks out from the secret's
/// position in every direction, a 36x36x36 box). Add an entry here for any secret that needs a
/// different reveal range than its category's default.
const OVERRIDES: &[(&str, &str, f64)] = &[
    // Red Green's "s2" item drop (the itemsp entry at 1,61,15 in secrets.json) needs a wider
    // reveal range than the default 16x16x16 box - per explicit request, 18 blocks out in every
    // direction (radius 18.0) instead of the usual 8.0. The room's other item drop, "s3", is
    // unaffected and keeps the default.
    ("Red Green", "s2", 18.0),
    // Supertall's lower bat - the batsp entry "s1" at (7, 71, 4) - needs its box extended by
    // three blocks per explicit request (11.0 instead of the usual 8.0 radius). The room's other
    // bat secret, batdie "s3" at (46, 116, 8), is much higher up and unaffected.
    ("Supertall", "s1", 11.0),
];

/// Returns the overridden spawn radius for `(room_name, secret_id)` if one is registered above,
/// otherwise `default_radius` (the calling category's normal value) unchanged.
pub fn spawn_radius_for(room_name: &str, secret_id: &str, default_radius: f64) -> f64 {
    OVERRIDES.iter()
        .find(|(room, id, _)| *room == room_name && *id == secret_id)
        .map(|(_, _, radius)| *radius)
        .unwrap_or(default_radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overridden_secret_uses_its_registered_radius() {
        assert_eq!(spawn_radius_for("Red Green", "s2", 8.0), 18.0);
    }

    #[test]
    fn same_room_different_id_falls_back_to_default() {
        assert_eq!(spawn_radius_for("Red Green", "s3", 8.0), 8.0);
    }

    #[test]
    fn same_id_different_room_falls_back_to_default() {
        assert_eq!(spawn_radius_for("Spider", "s2", 8.0), 8.0);
    }

    #[test]
    fn unregistered_room_and_id_falls_back_to_default() {
        assert_eq!(spawn_radius_for("Some Other Room", "s7", 4.0), 4.0);
    }

    #[test]
    fn supertall_lower_bat_gets_extended_radius() {
        assert_eq!(spawn_radius_for("Supertall", "s1", 8.0), 11.0);
    }
}

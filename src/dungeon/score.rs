//! Hypixel SkyBlock Catacombs F7 score tracking.
//!
//! The four category formulas (`skill_score`/`exploration_score`/`speed_score`/`bonus_score`)
//! are plain functions with no dependency on `Dungeon`/`World` - easy to unit-test/tune in
//! isolation. `DungeonScoreState` is the per-run mutable state; `sync_exploration` recomputes
//! its room/secret fields from the dungeon's own `Room` data each call rather than tracking a
//! second, independently-incremented copy, since `Room::starred_mobs_remaining`/`found_secrets`
//! are already the authoritative live source for "is this room cleared"/"how many secrets".
//!
//! `deaths`/`failed_puzzles`/`mimic_killed`/`paul_ezpz` have no real trigger in this codebase
//! yet (no player damage/death system, no puzzle minigames, no mimic entity) - they're plain
//! fields with public setters (see `Dungeon::record_death` etc in `dungeon.rs`) ready to be
//! wired up once those systems exist, plus a `/dscore` debug command so the score breakdown
//! itself can be exercised without them.

use crate::dungeon::room::room::Room;
use crate::dungeon::room::room_data::RoomType;

const F7_FULL_SPEED_TICKS: u64 = 14 * 60 * 20;

pub fn skill_score(deaths: u32, failed_puzzles: u32, spirit_first_death: bool) -> u32 {
    let mut penalty = deaths * 2;

    if deaths > 0 && spirit_first_death {
        penalty = penalty.saturating_sub(1);
    }

    100u32
        .saturating_sub(penalty + failed_puzzles * 14)
        .max(20)
}

pub fn exploration_score(
    cleared_rooms: u32,
    total_rooms: u32,
    secrets_found: u32,
    total_secrets: u32,
) -> u32 {
    let room_score = if total_rooms == 0 {
        0
    } else {
        (60 * cleared_rooms) / total_rooms
    };

    let secret_score = if total_secrets == 0 {
        40
    } else {
        ((40 * secrets_found) / total_secrets).min(40)
    };

    (room_score + secret_score).min(100)
}

pub fn speed_score(elapsed_ticks: u64) -> u32 {
    if elapsed_ticks <= F7_FULL_SPEED_TICKS {
        return 100;
    }

    // Approximate fallback: lose 1 point every 16.8 seconds immediately after 14:00.
    let overtime_ticks = elapsed_ticks - F7_FULL_SPEED_TICKS;
    let penalty = overtime_ticks / 336; // 16.8 sec * 20 ticks

    100u32.saturating_sub(penalty as u32)
}

pub fn bonus_score(crypts: u32, mimic_killed: bool, paul_ezpz: bool) -> u32 {
    crypts.min(5)
        + if mimic_killed { 2 } else { 0 }
        + if paul_ezpz { 10 } else { 0 }
}

#[derive(Default, Debug, Clone)]
pub struct DungeonScoreState {
    pub deaths: u32,
    pub failed_puzzles: u32,
    pub spirit_first_death: bool,

    pub cleared_rooms: u32,
    pub total_rooms: u32,
    pub secrets_found: u32,
    pub total_secrets: u32,

    pub elapsed_ticks: u64,

    pub crypts: u32,
    pub mimic_killed: bool,
    pub paul_ezpz: bool,
}

impl DungeonScoreState {
    /// Resets all per-run state - call when a new dungeon run starts.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn skill(&self) -> u32 {
        skill_score(self.deaths, self.failed_puzzles, self.spirit_first_death)
    }

    pub fn exploration(&self) -> u32 {
        exploration_score(self.cleared_rooms, self.total_rooms, self.secrets_found, self.total_secrets)
    }

    pub fn speed(&self) -> u32 {
        speed_score(self.elapsed_ticks)
    }

    pub fn bonus(&self) -> u32 {
        bonus_score(self.crypts, self.mimic_killed, self.paul_ezpz)
    }

    pub fn total(&self) -> u32 {
        self.skill() + self.exploration() + self.speed() + self.bonus()
    }

    /// Recomputes `cleared_rooms`/`total_rooms`/`secrets_found`/`total_secrets` from live room
    /// data. Entrance/Fairy rooms are excluded, matching `map.rs`'s own checkmark exclusion
    /// (they have no clear objective and are never checked off on the map either).
    pub fn sync_exploration(&mut self, rooms: &[Room]) {
        let mut cleared = 0u32;
        let mut total_rooms = 0u32;
        let mut secrets_found = 0u32;
        let mut total_secrets = 0u32;

        for room in rooms {
            if matches!(room.room_data.room_type, RoomType::Entrance | RoomType::Fairy) {
                continue;
            }
            total_rooms += 1;
            if room.starred_mobs_remaining == 0 {
                cleared += 1;
            }
            secrets_found += room.found_secrets as u32;
            total_secrets += room.room_data.secrets as u32;
        }

        self.cleared_rooms = cleared;
        self.total_rooms = total_rooms;
        self.secrets_found = secrets_found;
        self.total_secrets = total_secrets;
    }
}

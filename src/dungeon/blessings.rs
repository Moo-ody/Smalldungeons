//! Dungeon Blessings - the 4 random buffs a "blessing chest" can grant (Tic Tac Toe, Ice Fill,
//! Ice Path, Boulder, Teleport Maze all reuse the same `DungeonSecret::blessing_texture`/
//! `EssenceEntityImpl` mechanism for their own reward chest - see those files). Quiz's own
//! "Blessing of Time" is a separate, fixed (always level V, never levels up further) reward
//! handled directly in `quiz.rs` - not part of this module.
//!
//! Confirmed real data, two independent sources:
//! - The exact chat message text/colors for a real "Blessing of Life V" pickup (user-provided,
//!   from a live screenshot) - quoted verbatim in `format_pickup_message`'s doc comment. Only
//!   this one blessing's message has been verified against a real capture; Power/Stone/Wisdom
//!   reuse the same template with the confirmed stat colors/icons and confirmed formula, but
//!   their exact message text has not been independently captured.
//! - The scaling formula and the 4 real types (Odin's own `Blessing` enum in `DungeonEnums.kt`,
//!   which reads blessing level as a Roman numeral straight off the real tab-list footer -
//!   confirming levels are a real, persistent, incrementing value, not per-pickup) plus
//!   `hypixelskyblock.minecraft.wiki`'s "Dungeon Blessing" page for the exact per-stat rates:
//!   flat = level × flat_rate, percent = level × percent_rate, and a single confirmed
//!   floor-dependent multiplier - "Blessings are 20% more effective on Floor III or higher" -
//!   applied uniformly to both components. This project has no floor-tracking concept at all
//!   (checked `Dungeon` directly - no such field exists), so `FLOOR_EFFECT_MULTIPLIER` is just
//!   applied unconditionally, matching a Floor III+ / high-Catacombs-level run.
//!
//! Deliberately does NOT track a per-run level for each type - every pickup is announced as a
//! fresh level 1 (see `FIXED_LEVEL`).

/// One of the 4 randomly-found blessing types (`Blessing of Time` is separate - see this
/// module's doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlessingKind {
    Life,
    Power,
    Stone,
    Wisdom,
}

impl BlessingKind {
    pub const ALL: [BlessingKind; 4] = [BlessingKind::Life, BlessingKind::Power, BlessingKind::Stone, BlessingKind::Wisdom];

    pub fn display_name(&self) -> &'static str {
        match self {
            BlessingKind::Life => "Life",
            BlessingKind::Power => "Power",
            BlessingKind::Stone => "Stone",
            BlessingKind::Wisdom => "Wisdom",
        }
    }

    /// The stats this blessing buffs, in the order they're announced. Rates are the confirmed
    /// per-level base amounts (before the floor multiplier) - see this module's doc comment.
    fn stats(&self) -> &'static [StatBuff] {
        match self {
            // Health/Health Regen are percent-only - confirmed directly from the real captured
            // message, which never mentions a flat component for either.
            BlessingKind::Life => &[
                StatBuff { name: "HP", icon: None, color: 'c', percent_per_level: 0.03, flat_per_level: 0.0 },
                // Icon color 'c' (red) confirmed directly from the real captured message
                // ("§a+1.21x §c❤ §aHealth Regen") - the heart is red even though the
                // surrounding text (hardcoded in `format_pickup_message`) is green.
                StatBuff { name: "Health Regen", icon: Some('\u{2764}'), color: 'c', percent_per_level: 0.03, flat_per_level: 0.0 },
            ],
            BlessingKind::Power => &[
                StatBuff { name: "Strength", icon: Some('\u{2741}'), color: 'c', percent_per_level: 0.02, flat_per_level: 4.0 },
                StatBuff { name: "Crit Damage", icon: Some('\u{2620}'), color: '1', percent_per_level: 0.02, flat_per_level: 4.0 },
            ],
            // Damage has no percent component (confirmed: the wiki's formula only lists a flat
            // rate for it), unlike every other stat here.
            BlessingKind::Stone => &[
                StatBuff { name: "Defense", icon: Some('\u{2748}'), color: 'a', percent_per_level: 0.02, flat_per_level: 4.0 },
                StatBuff { name: "Damage", icon: None, color: 'c', percent_per_level: 0.0, flat_per_level: 6.0 },
            ],
            BlessingKind::Wisdom => &[
                StatBuff { name: "Intelligence", icon: Some('\u{270E}'), color: '9', percent_per_level: 0.02, flat_per_level: 4.0 },
                StatBuff { name: "Speed", icon: Some('\u{2726}'), color: 'f', percent_per_level: 0.02, flat_per_level: 4.0 },
            ],
        }
    }
}

struct StatBuff {
    name: &'static str,
    /// `None` for a stat shown as a plain abbreviation with no icon (matches the real captured
    /// message: "HP" has no icon at all, only "Health Regen" gets the heart).
    icon: Option<char>,
    /// Legacy color code character (e.g. `'c'` for `§c`) for this stat's own icon/accent -
    /// the confirmed real per-stat icon colors: health/strength red, crit damage dark blue,
    /// defense green, intelligence blue, speed white.
    color: char,
    percent_per_level: f64,
    flat_per_level: f64,
}

/// "Blessings are 20% more effective on Floor III or higher" - the one confirmed
/// floor-dependent multiplier (see this module's doc comment). Applied unconditionally since
/// this project has no floor concept to gate it on.
const FLOOR_EFFECT_MULTIPLIER: f64 = 1.2;

/// Every pickup is announced as a fresh level 1 - deliberately no persistent per-run level
/// tracking (real Hypixel blessings level up the more of a given type you find across a run,
/// but this project doesn't track that).
pub const FIXED_LEVEL: u32 = 1;

fn to_roman(mut level: u32) -> String {
    const NUMERALS: [(u32, &str); 13] = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"),
        (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
        (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    let mut result = String::new();
    for &(value, numeral) in &NUMERALS {
        while level >= value {
            result.push_str(numeral);
            level -= value;
        }
    }
    result
}

/// Builds the real 2-line "DUNGEON BUFF!" announcement for finding `kind` at `level`.
///
/// The exact text/colors below are verified against a real captured `Blessing of Life V`
/// message:
/// ```text
/// §6§lDUNGEON BUFF! §fYou found a §dBlessing of Life V§f!
/// §7Granted you §a+1.21x HP §7and §a+1.21x §c❤ §aHealth Regen§7.
/// ```
/// Power/Stone/Wisdom reuse this exact template (same colors, same "Granted you X and Y."
/// structure) with their own confirmed stat names/icons/colors substituted in - not
/// independently verified against a live capture of those specific messages. For a stat with
/// both a percent and a flat component (every stat except Life's, and Stone's Damage), the flat
/// bonus is appended in gray parentheses after the same "+X.XXx" multiplier phrasing, since no
/// real example of that combined case was available to confirm an exact format against.
pub fn format_pickup_message(kind: BlessingKind, level: u32) -> [String; 2] {
    let roman = to_roman(level);
    let line1 = format!(
        "\u{a7}6\u{a7}lDUNGEON BUFF! \u{a7}fYou found a \u{a7}dBlessing of {} {}\u{a7}f!",
        kind.display_name(), roman
    );

    let stats = kind.stats();
    let parts: Vec<String> = stats.iter().map(|stat| {
        let percent = level as f64 * stat.percent_per_level * FLOOR_EFFECT_MULTIPLIER;
        let flat = level as f64 * stat.flat_per_level * FLOOR_EFFECT_MULTIPLIER;

        let icon_part = match stat.icon {
            Some(icon) => format!("\u{a7}{}{icon} ", stat.color),
            None => String::new(),
        };

        if stat.percent_per_level > 0.0 && stat.flat_per_level > 0.0 {
            format!("\u{a7}a+{:.2}x {icon_part}\u{a7}a{} \u{a7}7(\u{a7}a+{:.0}\u{a7}7)", 1.0 + percent, stat.name, flat)
        } else if stat.percent_per_level > 0.0 {
            format!("\u{a7}a+{:.2}x {icon_part}\u{a7}a{}", 1.0 + percent, stat.name)
        } else {
            format!("\u{a7}a+{:.0} {icon_part}\u{a7}a{}", flat, stat.name)
        }
    }).collect();

    let line2 = format!("\u{a7}7Granted you {} \u{a7}7and {}\u{a7}7.", parts[0], parts[1]);

    [line1, line2]
}

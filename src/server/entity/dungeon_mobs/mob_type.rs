//! Reusable dungeon mob archetypes.
//!
//! The room mob JSON files (`src/room_data/mobs/*.json`) contain thousands of individual
//! mob *spawn entries*, but each entry's `fullName` is drawn from a small, fixed set of
//! recurring mob archetypes (e.g. "Crypt Lurker", "Super Tank Zombie"). [`DungeonMobType`]
//! is that reusable set - it is the "type" a spawn entry maps to, while the JSON itself only
//! ever supplies placement data (position, rotation, equipment, star flag, skin).

/// The underlying vanilla mob model/behavior family an archetype is displayed as.
/// Mirrors the JSON `mobType` field (`"zombie"`, `"skeleton"`, `"enderman"`, `"player"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MobBaseKind {
    Zombie,
    Skeleton,
    Enderman,
    Player,
}

impl MobBaseKind {
    pub fn from_json_str(value: &str) -> Option<Self> {
        match value {
            "zombie" => Some(Self::Zombie),
            "skeleton" => Some(Self::Skeleton),
            "enderman" => Some(Self::Enderman),
            "player" => Some(Self::Player),
            _ => None,
        }
    }
}

/// The ~19 recurring Catacombs mob archetypes found across every room's mob data.
/// Every one of the thousands of spawn entries maps to one of these variants by `fullName` -
/// none of them get their own bespoke struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DungeonMobType {
    SuperTankZombie,
    ZombieSoldier,
    ZombieKnight,
    ZombieLord,
    ZombieCommander,
    CryptLurker,
    CryptDreadlord,
    CryptSouleater,
    CryptUndead,
    SkeletonSoldier,
    SkeletonMaster,
    SkeletonLord,
    SuperArcher,
    Sniper,
    Withermancer,
    Fels,
    AngryArchaeologist,
    LostAdventurer,
    FrozenAdventurer,
}

impl DungeonMobType {
    /// Maps a spawn entry's `fullName` to its archetype.
    /// Returns `None` for any name that isn't one of the known archetypes, so unrecognized
    /// (e.g. future) mob names don't need code changes - callers fall back to the JSON
    /// `mobType` field instead of failing.
    pub fn from_full_name(full_name: &str) -> Option<Self> {
        Some(match full_name {
            "Super Tank Zombie" => Self::SuperTankZombie,
            "Zombie Soldier" => Self::ZombieSoldier,
            "Zombie Knight" => Self::ZombieKnight,
            "Zombie Lord" => Self::ZombieLord,
            "Zombie Commander" => Self::ZombieCommander,
            "Crypt Lurker" => Self::CryptLurker,
            "Crypt Dreadlord" => Self::CryptDreadlord,
            "Crypt Souleater" => Self::CryptSouleater,
            "Crypt Undead" => Self::CryptUndead,
            "Skeleton Soldier" => Self::SkeletonSoldier,
            "Skeleton Master" => Self::SkeletonMaster,
            "Skeleton Lord" => Self::SkeletonLord,
            "Super Archer" => Self::SuperArcher,
            "Sniper" => Self::Sniper,
            "Withermancer" => Self::Withermancer,
            "Fels" => Self::Fels,
            "Angry Archaeologist" => Self::AngryArchaeologist,
            "Lost Adventurer" => Self::LostAdventurer,
            "Frozen Adventurer" => Self::FrozenAdventurer,
            _ => return None,
        })
    }

    /// The model/entity family this archetype is spawned as. This is the "model" half of
    /// what the archetype defines - equipment and position always still come from the JSON.
    pub const fn base_kind(&self) -> MobBaseKind {
        match self {
            Self::SuperTankZombie
            | Self::ZombieSoldier
            | Self::ZombieKnight
            | Self::ZombieLord
            | Self::CryptLurker => MobBaseKind::Zombie,

            Self::SkeletonSoldier
            | Self::SkeletonMaster
            | Self::SkeletonLord
            | Self::SuperArcher
            | Self::Sniper
            | Self::Withermancer => MobBaseKind::Skeleton,

            Self::Fels => MobBaseKind::Enderman,

            Self::ZombieCommander
            | Self::CryptDreadlord
            | Self::CryptSouleater
            | Self::CryptUndead
            | Self::AngryArchaeologist
            | Self::LostAdventurer
            | Self::FrozenAdventurer => MobBaseKind::Player,
        }
    }

    /// Base health shown on the mob's nametag. `None` for archetypes whose real HP isn't known
    /// yet (rather than guessing) - the nametag just omits the health suffix for those.
    pub const fn base_health(&self) -> Option<u64> {
        match self {
            Self::CryptLurker => Some(420_000),
            Self::CryptUndead => Some(45_000),
            Self::CryptDreadlord => Some(840_000),
            Self::SuperTankZombie => Some(90_000),
            Self::ZombieSoldier => Some(1_200_000),
            Self::ZombieKnight => Some(1_280_000),
            Self::ZombieCommander => Some(3_500_000),
            Self::ZombieLord => Some(5_000_000),
            Self::SkeletonSoldier => Some(750_000),
            Self::SkeletonMaster => Some(900_000),
            Self::SkeletonLord => Some(2_000_000),
            Self::Sniper => Some(500_000),
            Self::CryptSouleater => Some(800_000),
            Self::Withermancer => Some(1_250_000),
            Self::SuperArcher => Some(1_600_000),
            Self::Fels => Some(1_500_000),
            Self::AngryArchaeologist | Self::LostAdventurer | Self::FrozenAdventurer => Some(2_000_000),
        }
    }

    /// Whether this archetype renders as a Wither Skeleton rather than a normal Skeleton.
    /// Only meaningful for archetypes whose `base_kind()` is [`MobBaseKind::Skeleton`].
    pub const fn is_wither_skeleton(&self) -> bool {
        matches!(self, Self::Withermancer)
    }

    /// Whether this archetype should render upside-down (the vanilla "Dinnerbone"/"Grumm"
    /// name trick - the entity's real custom name is set to that string but kept invisible,
    /// so only the flip effect applies and no name floats above it).
    pub const fn is_upside_down(&self) -> bool {
        matches!(self, Self::Fels)
    }

    /// Whether this `Player`-kind archetype should be spawned as a real player-model NPC
    /// (`SpawnPlayer` + tab-list skin registration) instead of falling back to the plain
    /// zombie model the other `Player`-kind archetypes use. Only enabled for archetypes
    /// explicitly confirmed as wanting the full NPC treatment.
    pub const fn spawn_as_npc(&self) -> bool {
        matches!(
            self,
            Self::LostAdventurer
                | Self::ZombieCommander
                | Self::FrozenAdventurer
                | Self::CryptDreadlord
                | Self::CryptSouleater
                | Self::CryptUndead
                | Self::AngryArchaeologist
        )
    }

    /// Explicit skin override (Mojang profile "textures" property value + signature) for a
    /// `spawn_as_npc` archetype. `None` means "use the default placeholder skin" - fill in a
    /// `Some((value, signature))` arm per archetype here once real skins are chosen; nothing
    /// else needs to change, the spawner already falls back to this automatically.
    pub const fn skin_override(&self) -> Option<(&'static str, &'static str)> {
        None
    }
}

/// Formats a raw HP number the way Hypixel dungeon nametags do: `3,500,000` -> `"3.5M"`,
/// `90,000` -> `"90k"`. Trims to at most 2 decimal places and drops trailing zeros.
pub fn format_health(hp: u64) -> String {
    fn trimmed(value: f64) -> String {
        let s = format!("{value:.2}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }

    if hp >= 1_000_000 {
        format!("{}M", trimmed(hp as f64 / 1_000_000.0))
    } else if hp >= 1_000 {
        format!("{}k", trimmed(hp as f64 / 1_000.0))
    } else {
        hp.to_string()
    }
}

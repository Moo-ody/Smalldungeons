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
    /// Not spawned from room JSON like the others - one random locked chest per dungeon is
    /// swapped for a trapped chest that spawns this in on open (see
    /// `spawner::spawn_mimic`/`main.rs`'s post-locked-chest-spawn selection step).
    Mimic,
    /// Not spawned from room JSON either - spawned ad hoc at the position of King Midas's
    /// golden "crypt" once a player superbooms it (see `spawner::spawn_king_midas`/
    /// `Room::explode_kingmidas_near`). Has no real HP: `ai/combat.rs::apply_king_midas_hit`
    /// kills him outright on his 5th hit (each of the first 4 strips one piece of armor)
    /// instead of using the lethal-weapon-only path every other archetype relies on.
    KingMidas,
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
            | Self::CryptLurker
            | Self::Mimic => MobBaseKind::Zombie,

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
            | Self::FrozenAdventurer
            | Self::KingMidas => MobBaseKind::Player,
        }
    }

    /// Base health shown on the mob's nametag. `None` for archetypes whose real HP isn't known
    /// yet (rather than guessing) - the nametag just omits the health suffix for those.
    pub const fn base_health(&self) -> Option<u64> {
        match self {
            Self::CryptLurker => Some(420_000),
            Self::CryptUndead => Some(500_900),
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
            Self::Mimic => Some(1_500_000),
            // Shown on his nametag like every other archetype's HP - `ai/combat.rs`'s hit
            // handler divides this by `KING_MIDAS_HITS_TO_KILL` and re-displays the remainder
            // after each hit, even though he actually dies from a fixed hit count, not damage.
            Self::KingMidas => Some(4_300_000),
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
                | Self::KingMidas
        )
    }

    /// Explicit skin override (Mojang profile "textures" property value + optional signature)
    /// for a `spawn_as_npc` archetype. `None` means "use the default placeholder skin" - fill
    /// in a `Some((value, signature))` arm per archetype here once real skins are chosen;
    /// nothing else needs to change, the spawner already falls back to this automatically.
    ///
    /// The signature is only meaningful for textures actually hosted on
    /// `textures.minecraft.net` and signed by Mojang's Yggdrasil key - a signature can't be
    /// forged for arbitrary/third-party texture URLs, so those must use `None` here.
    ///
    /// Unlike skulls elsewhere in this codebase (Mimic/Sniper/Fels/Blood Key equipment, all
    /// confirmed working with an unsigned `ItemStack::set_skull_owner` property), a real
    /// `SpawnPlayer`-model NPC's skin apparently DOES need a valid signature to actually render
    /// - confirmed by testing (`CryptUndead`'s skin silently fell back to the default skin with
    /// `None` here, and started working once given a real signed property for the exact same
    /// texture, looked up via mineskin.org's API). So in practice `None` should only be used
    /// here as a genuinely last-resort placeholder, not a routine choice for a hash-only
    /// texture like it safely is for skulls.
    pub const fn skin_override(&self) -> Option<(&'static str, Option<&'static str>)> {
        match self {
            Self::CryptDreadlord | Self::CryptSouleater => Some((
                CRYPT_DREADLORD_SOULEATER_SKIN,
                Some(CRYPT_DREADLORD_SOULEATER_SKIN_SIGNATURE),
            )),
            Self::ZombieCommander => Some((
                ZOMBIE_COMMANDER_SKIN,
                Some(ZOMBIE_COMMANDER_SKIN_SIGNATURE),
            )),
            Self::CryptUndead => Some((CRYPT_UNDEAD_SKIN, Some(CRYPT_UNDEAD_SKIN_SIGNATURE))),
            Self::FrozenAdventurer => Some((
                FROZEN_ADVENTURER_SKIN,
                Some(FROZEN_ADVENTURER_SKIN_SIGNATURE),
            )),
            Self::AngryArchaeologist => Some((
                ANGRY_ARCHAEOLOGIST_SKIN,
                Some(ANGRY_ARCHAEOLOGIST_SKIN_SIGNATURE),
            )),
            Self::KingMidas => Some((KING_MIDAS_SKIN, Some(KING_MIDAS_SKIN_SIGNATURE))),
            _ => None,
        }
    }
}

/// Per-spawn body-skin override for Lost Adventurer, keyed by the JSON equipment's helmet
/// display name. Unlike every other archetype, "Lost Adventurer" covers four distinct armor
/// sets sharing one `fullName` and one set of AI/combat stats - Young Dragon, Holy Dragon,
/// Superior Dragon, and Unstable Dragon - found by scanning every room's mob JSON for
/// `fullName == "Lost Adventurer"` and diffing equipment. Young and Superior happen to share
/// the exact same underlying body-skin texture hash, leaving 3 distinct skins across the 4
/// armor variants.
///
/// The scraped room JSON already carries each spawn's own `skin` value (`MobSpawnJson::skin`),
/// but with no `signature` alongside it - an apparent gap in how the data was originally
/// scraped. A signed player-model skin needs a real signature to render at all (see
/// `skin_override` above - an unsigned one silently falls back to the default Steve/Alex
/// skin), so all 3 were re-resolved from the same texture hash already embedded in that `skin`
/// value via mineskin.org's `/v2/generate` (URL-based, no local file needed since Mojang
/// already hosts the texture) - same lookup approach as `CRYPT_UNDEAD_SKIN`/
/// `FROZEN_ADVENTURER_SKIN` above, just keyed by hash instead of by re-uploading a PNG.
///
/// Called from `spawner::spawn_single_mob`, which is the only place with access to the current
/// spawn's own equipment (this can't live in `skin_override` above, which is a pure function of
/// the archetype alone and has no per-spawn JSON to key off of).
pub fn lost_adventurer_skin_for_helmet(helmet_name: &str) -> Option<(&'static str, &'static str)> {
    match helmet_name {
        "Young Dragon Helmet" | "Superior Dragon Helmet" => Some((
            LOST_ADVENTURER_YOUNG_SUPERIOR_SKIN,
            LOST_ADVENTURER_YOUNG_SUPERIOR_SKIN_SIGNATURE,
        )),
        "Holy Dragon Helmet" => Some((LOST_ADVENTURER_HOLY_SKIN, LOST_ADVENTURER_HOLY_SKIN_SIGNATURE)),
        "Unstable Dragon Helmet" => Some((
            LOST_ADVENTURER_UNSTABLE_SKIN,
            LOST_ADVENTURER_UNSTABLE_SKIN_SIGNATURE,
        )),
        _ => None,
    }
}

/// GameProfile "textures" property value shared by the Young Dragon and Superior Dragon Lost
/// Adventurer body skins (Mojang profile "__notahuman__" - both armor variants render the same
/// underlying body/head skin), plus its Yggdrasil signature.
const LOST_ADVENTURER_YOUNG_SUPERIOR_SKIN: &str = "ewogICJ0aW1lc3RhbXAiIDogMTYxMzAyNTcxNjYyMiwKICAicHJvZmlsZUlkIiA6ICI2MTZiODhkNDMwNzM0ZTM3OWM3NDc1ODdlZTJkNzlmZCIsCiAgInByb2ZpbGVOYW1lIiA6ICJfX25vdGFodW1hbl9fIiwKICAic2lnbmF0dXJlUmVxdWlyZWQiIDogdHJ1ZSwKICAidGV4dHVyZXMiIDogewogICAgIlNLSU4iIDogewogICAgICAidXJsIiA6ICJodHRwOi8vdGV4dHVyZXMubWluZWNyYWZ0Lm5ldC90ZXh0dXJlLzM2YTAzODNhNTI3ODAzZDk5YjY2MmFkMThiY2FjNzhjMTE5MjUwZWJiZmIxNDQ3NWI0ZWI0ZDRhNjYyNzk2YjQiCiAgICB9CiAgfQp9";
const LOST_ADVENTURER_YOUNG_SUPERIOR_SKIN_SIGNATURE: &str = "whMdXQmOnSvOHeS2G8mIiTyeYKl+o6H0dreFEthtYbUrIt0IhogVYMjDC15pK4Qw6hYsfbu0Mc5254Gpkfnp0/Sdtz12Id4xDlL4zLDqNtlY2RGiOHk8keuuvTzLn1kKxubOodzeIvX8r2HWcjWqYmtTzMAF4DqN0r1A/TKC5xRQ4EDRzbahV9mRa2hm8OFsIQxbyL45uYAzkB5BM7JNh/8/VFTXtkE76flEex9SEqumMiisIf7fxyo8Gfnh/3jFmqR054QJ5siJ5cWapSKzB5sF6GEz2x+EHh01OKBKCECWITaeruXT866+wFkjR/ffdkY9VzO7GAbYmJ8GfZR9Bxt1tppTeWvPVf+u4cxZC7L5IYT2wSDcFR7qXy49SvKEczzZCJNEF23daAHDXJ/mJnw8RjiB6zXfbgEnMGkQP3MgClVal9sXXsgvAubUb+Bgo94m/q6EKGnfdvnVMpEAgTUFqgLUIvMRhDSGGgA+i5hS+f03Y8Aq8s5VOnRguqgJhgdk3uUEDnYteJxymTbGiUnX1gxxRIifCSSKNbTzCP7GN8V+LFxas8mLtSgKL+rQxCq/+iaSYBKVGtTjJFMHRTXT1E4QZCQxUgW2WcP3M1gT1Y1bJqdEJDS5w0k6Q8If97VjwAV24VFvQcHSpnZnuLfdYts/KY/WIsk499XQXu4=";

/// GameProfile "textures" property value for the Holy Dragon Lost Adventurer body skin (Mojang
/// profile "Tompkin42"), plus its Yggdrasil signature.
const LOST_ADVENTURER_HOLY_SKIN: &str = "ewogICJ0aW1lc3RhbXAiIDogMTYxNzcxMjI2MTEwNCwKICAicHJvZmlsZUlkIiA6ICI5ZDQyNWFiOGFmZjg0MGU1OWM3NzUzZjc5Mjg5YjMyZSIsCiAgInByb2ZpbGVOYW1lIiA6ICJUb21wa2luNDIiLAogICJzaWduYXR1cmVSZXF1aXJlZCIgOiB0cnVlLAogICJ0ZXh0dXJlcyIgOiB7CiAgICAiU0tJTiIgOiB7CiAgICAgICJ1cmwiIDogImh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvM2EyNmNmNjZhZjQyYTQ4ZjZiZTUyYTBhNDhhOGViNTk4ZjNhMjM2MzliN2E1YThlZTE3MWMxMzk5OWVlOTRjOSIKICAgIH0KICB9Cn0=";
const LOST_ADVENTURER_HOLY_SKIN_SIGNATURE: &str = "TOC2JuQ0nZYaN8LdAZnYxzzeJwcPfBgeBhtU9NQoPQE44bo/COdbsD/B7fLqd7GVa1FiGDTEEtgBGd9J18ybB6oMPYtD7sU+PamQEsu/SJiOz6haTQy/kFGBv+KaWie2L9Fr495O5CrXrTMzPCYl4WAvxBfqcWU3ZJ6KOCvT/oNhTGz1xOYrxuAupsvUFFPcsPUy5GxqPT9jW5TS4pCbr3RI6nKB1j7iSWk5+MRaSUkxPdkoGqneP6JNE7gOOI5rio5g1aP6yYwbQDr3+w77laEDTJ5a3DHqdwk05VggIcOvgL4+pCx4cBPsh7yQqu+Yka8whtUAb3A+UNRl6jfbswQyFfCrrj5FwGXXh2XYFarlOQy/yKg+5Ox2djhYOFbnG9Cy9F6Mn8DeAyHwtO3sLVMHyS2zFpznkvfNLuqYiOuoJ6oLr5WcP1Hihxau+7iNPbDldfgL4iunkIXnpIXRGiuhkbMDLiMa55q5t54SS3ypE20RSfRugHUVLlob4Otsvk2NembWYLcspNsVWxCSB+dlIr8mvEakMkJaLPQNDkf+m6UZ3Wjhb3m/zndEKRZHqCmmsPplCwmkIDXRgupOXsgY3PJtxcRwaCztji8ImWNUzKPcUZ2sGp0TT+gzqRWvzuS9mjUyxPo2r8EvMiANqUfW7w0U3/wMbtdc+C5XN4s=";

/// GameProfile "textures" property value for the Unstable Dragon Lost Adventurer body skin
/// (Mojang profile "QuantumBlocker"), plus its Yggdrasil signature.
const LOST_ADVENTURER_UNSTABLE_SKIN: &str = "ewogICJ0aW1lc3RhbXAiIDogMTczOTMxMTAyMTYyOSwKICAicHJvZmlsZUlkIiA6ICJkYjZiYWRlN2NjMzI0MjM4YjU3OTQ4NzMxNTBkNjA1MiIsCiAgInByb2ZpbGVOYW1lIiA6ICJRdWFudHVtQmxvY2tlciIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9lMzgxYTE4NzVlZmViMmEyNjBkMzU3OTg0YzNlNzVhN2RmYmM3YTIyYjU4MmJmZTk5OTA0MjFjYmI0NDVhY2MiCiAgICB9CiAgfQp9";
const LOST_ADVENTURER_UNSTABLE_SKIN_SIGNATURE: &str = "BWG0CSG/YmMTWffAM5G8CwCvV2LpbSd7Gt/j+Ybbjowd7VP4LH1C8DoFNVDLSGfw/d7Xq8FZP/PolNBRle4YhhpTeQlJJLbKgyZtj0IgSdGejqy4OCAafcybuzc+96/2e9yHm0pULh6PrK4Dy4vXNHAU/vOw8c7WSP3TT7/9uRzovtgIksLeuVooRLg0ZcQ0BN08/dzKKvWa2789VT1IeUnLHG0Z41rhE+QF3F+hEP0QjJeshdfB6xipasXKKRbNQ63veq6e+fD74GTwgX7ceRfvHj/CtNtPjSWZqk3Qmtq/7lAPTxl5fnHS28szWWJCY7LaLt1Qs4TVbTFufuv6XhFYRi+z2jYudTvJMeDWS68SPaeeRJXtbKE/NwMS068y/Xqm59gHQQiJGQEgmTrd1DLsVkpJxVXp2DOOMCVFIWkG/F1AjuWlmWOn/Lpb4JhijI5/tQg/NAEVvcYgwgawnqoUs8oJFfSV3eJWS1W7gOATV79veYvzznJkp6+N61te5yuTInNZb6J6W1p5NqkqQyh9rz122AGODq500sZhZ6BQdHVjW1LuF7xJ54c6qqRgfi1QW/C6Mwu0Cs2cP4jd9H5MwYf/a0zWjqJYd5lxD+DFmKumxPpQ6vFk9AgXaNRrNUPhe4P0cH4t8dlHBmvql27Dw6qHc8ABQJPdd/oD76M=";

/// GameProfile "textures" property value shared by the Crypt Dreadlord and Crypt Souleater
/// NPC skins (Mojang profile "Yeleha"), plus its Yggdrasil signature.
const CRYPT_DREADLORD_SOULEATER_SKIN: &str = "ewogICJ0aW1lc3RhbXAiIDogMTYyNjMyMjczMDkzMSwKICAicHJvZmlsZUlkIiA6ICIzZmM3ZmRmOTM5NjM0YzQxOTExOTliYTNmN2NjM2ZlZCIsCiAgInByb2ZpbGVOYW1lIiA6ICJZZWxlaGEiLAogICJzaWduYXR1cmVSZXF1aXJlZCIgOiB0cnVlLAogICJ0ZXh0dXJlcyIgOiB7CiAgICAiU0tJTiIgOiB7CiAgICAgICJ1cmwiIDogImh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZDA5M2Q2NTJjMWI5ZjIyZmUwZTgxZWIxYTAyOGZhNGIwYjY5MDRjZWQ1YzdlZTlkNjI5YTgxOTU2MDc2NDk5YyIKICAgIH0KICB9Cn0=";
const CRYPT_DREADLORD_SOULEATER_SKIN_SIGNATURE: &str = "NybQzjteIreKG8mUVlpy4+gVYEloMFxsdAQyfRk5+WS2braPgTWfwxVvv8sukxJLpgxQjrOzSOVhwW5k4cO9j2n8ugWUOzUWnrxGzKmvegZ5UTmDVanLhg2ESFce0oFadJ7RrrQeYYgfqFFjKsA/9Q+Aky0KfdV38pt8U2UsGq68IVSjyickXD3QiwHR9u4FINT98th6m4/9iwhm80Oz1wd9C3O4kdpqGwNWrxLJx8MlcTfzmqSnuuw8bpSNXjXeD1yuScqAXkr8CYg78vg106YFQMNMuwNyIJX65HtTnjJD01xjoKVDw+jKZkFy9v/9ejtQyUjv1cumzrD+lQDejbKyFDNq5cuS0FGza3cfZrqXDXLRr4ujxARNQGxDsbRaXHVbGhuVnHfKy2Z5SjjPOgAzk+ZLzt3nINsp0lRj9xxYilOnKLi+6ExC38+1xUwcU2jtqvkqqCHYDe35WtVIj6nir/sBSbOu93z2anM7/eFH2cboGP/JVwrAJ4o5gH2u644DTxfB9zd6uUqs2mKGwSDd6N/S8IYJmjjQbk87mj9NpnMvWbPVpAs7pmROzuLJ12w+wJtUz6LqU1Nr5YgZyT2NgGiG9xZl560RAAXtNDexM29Zy+gNfIL6aYuLoy6Jz0OhPcKmDfsVWsSsUO7AQDRSLcc5cgGO17m/P0E0l6o=";

/// GameProfile "textures" property value for the Crypt Undead NPC skin (Mojang profile
/// "Relapago05"), plus its Yggdrasil signature - same shape as the other `spawn_as_npc`
/// skins below, unlike an earlier version of this constant which was an unsigned bare-hash
/// blob. That didn't render (confirmed by testing): unlike skulls elsewhere in this codebase
/// (Mimic/Sniper/Fels/Blood Key), which do render fine unsigned, a `SpawnPlayer`-model NPC's
/// skin apparently needs a real signature to actually display instead of silently falling back
/// to the default skin - every other `spawn_as_npc` archetype's skin here was already signed,
/// this was just the first one that wasn't. Looked up the same texture hash the user gave via
/// mineskin.org's public API, which had it cached from a previous real upload and returned the
/// actual signed Mojang profile for it.
const CRYPT_UNDEAD_SKIN: &str = "ewogICJ0aW1lc3RhbXAiIDogMTYyMjM0NjIyMDc3NSwKICAicHJvZmlsZUlkIiA6ICJmNDY0NTcxNDNkMTU0ZmEwOTkxNjBlNGJmNzI3ZGNiOSIsCiAgInByb2ZpbGVOYW1lIiA6ICJSZWxhcGFnbzA1IiwKICAic2lnbmF0dXJlUmVxdWlyZWQiIDogdHJ1ZSwKICAidGV4dHVyZXMiIDogewogICAgIlNLSU4iIDogewogICAgICAidXJsIiA6ICJodHRwOi8vdGV4dHVyZXMubWluZWNyYWZ0Lm5ldC90ZXh0dXJlLzMyZTJkMGVkMTRjMjM4NjI2NjZiNDA1YWVmYWJjNjVhNTdhMjcxZTg0MmZkYTVmNGQxOWU2YmMxMThjZTBjOTQiCiAgICB9CiAgfQp9";
const CRYPT_UNDEAD_SKIN_SIGNATURE: &str = "VUUssI0GBkkklcZLDAsot564Eev/cCjmIDi1jxt9KX5er8HYOriG+2wl3F0HxSIMESXEyqclJTQKCYfFqUl8OYpSQWSD1wTKx2aFU82i261dZGj5WJH3yWZSNR1CJpU0tLPQizPcCnJsoiQDAPSQn53iq2rZKqXU3Vz4nyNCdf+uTdfmX0iVmu8NWnpiz8huHevpmTXg9zMnWn7XgxtWDbuiAeAfEfcdEP+kgFYD6T81k0kFAEcUb0p7dNyISBBcpe515xRD+DQ91Zg9ThIKq00H/xdvgDwVZSHsYSLwLwn1qsVGj1tY5GKMfd2z6F59SuFhxz29D3Mplm5VLTEupM267RmK2skKOQp7mPdpI8r/XWVyCzmd6dRPwPLKh6GWtJCEUlhK//uc0W7J0t0gzoaG7tjgpUAmqYa3/O63fOBypJmYT80HSVeIpo2NAKTUkp3qd8pfXRDgujUpeZ0G3TBMRRK3XMPjIc1wBLwepkfsny+9+U2zmojuLOt7LwwvvfQnwGrMHtzrcAOfQQYNYAKmW2N012eqT/JuoTct/oUQ49Q3dO9dZtSWJVz0ysQJepLfKZLqcN0mjXR02OaApp+/l/zfE4qnx0RqKYFkGuswyF3wHP//S9gtE3XdOVo1lNkJS9TzXwA+31OSreuiMCwGtyBURSRu9ddsqpT00dQ=";

/// GameProfile "textures" property value for the Zombie Commander NPC skin (Mojang profile
/// "TheIndra"), plus its Yggdrasil signature.
const ZOMBIE_COMMANDER_SKIN: &str = "ewogICJ0aW1lc3RhbXAiIDogMTYyMzM2MDY3NzU3MCwKICAicHJvZmlsZUlkIiA6ICI1NjY3NWIyMjMyZjA0ZWUwODkxNzllOWM5MjA2Y2ZlOCIsCiAgInByb2ZpbGVOYW1lIiA6ICJUaGVJbmRyYSIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS81MDYxN2M1ZWY1NTJkMTA2OGNkYjA1ODg0ODU0YzA1YjI0MTEyOWI3ZWIxMjk5NjQwN2FkZWFjODA5M2FjYWJlIgogICAgfQogIH0KfQ==";
const ZOMBIE_COMMANDER_SKIN_SIGNATURE: &str = "E35WvyZCtobqGrHQQLWTaDyDZbt1nwTbImCJUAo4jWEYiyra2LX5WEzQF7rQTCVok2X73koBz6Ukw1n6oACKZGyGGFh95Vre44e68ieT4yuf1IGlpazAcNNhv4VKrDZxc9BteUc5/0HAuyiT7+waDhvnGHm68B+urQf5fj2+Wcqxyi3X6lykMTCct3eV/uM2CXv4/UL/UfnH3B9iaWNwPtAPX64wGLnQyEsU/GeysPna3YKHLT+V6el8g+/b504jiJH+CUhO9doaSJ/JdjCE6EIDK3PWVCHMbcB27kPeiN0mzNCL7tZsmsLgbBp4/IuFL7OJ2Cmqho8i8asjXhGsFTvC8p31ry0Cg45u7IrlsFUTQ2O8ynlR/PT3W9bomvKgjPSjEcr8SqTXxGJmu6nRy/8VWpZCdTS3Sn3aQ7T1udZVEEtIDGwOEU8HBRXJQDrOaR8NettACEcR2Umtejn3m+HA+aP79erVECtcRB3S9uoCKzXfutSjjlvFJgKHPpinsr6aUjF4EIAVcHr96sKS1BO/AImckIPmj3qqX86WIDYrSGdIYn20O4+SRdct0CjWlv+8ibDq50AogWSqaVU4ZSBwMsXUbXy7v6oYS94A3rOJUl/cTINOZDxnkFLkyWVqpATSUrSH+Q4UkepfNEqwIo5AYxONwC0/Lnng9DFCdjg=";

/// GameProfile "textures" property value for the Frozen Adventurer NPC body skin (Mojang
/// profile "GeyserMC"), plus its Yggdrasil signature. Sourced from a local PrismLauncher skin
/// cache file (`assets/skins/5e/5e31d404993fed975851cddaf8638eda089b952c` - that filename is a
/// SHA-1 of the PNG bytes, the client's own local cache key, NOT the Mojang CDN texture hash
/// this game actually needs) and resolved to the real signed profile via mineskin.org's
/// `/v2/generate` upload endpoint, which recognized it as an already-hosted texture
/// (`"duplicate": true` in the response) and returned its real hash/value/signature - same
/// `mineskin.org` lookup approach already used for `CRYPT_UNDEAD_SKIN` above.
const FROZEN_ADVENTURER_SKIN: &str = "ewogICJ0aW1lc3RhbXAiIDogMTYzMDMzNjkyMjE2NywKICAicHJvZmlsZUlkIiA6ICIyMWUzNjdkNzI1Y2Y0ZTNiYjI2OTJjNGEzMDBhNGRlYiIsCiAgInByb2ZpbGVOYW1lIiA6ICJHZXlzZXJNQyIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS8zMjYwMzI1MTcxYTdiYTg0NjA4MzBjMGVlYTUxNWM3NTdhNjY1ZTViMTZhMTQyMDdiYTFhMzE4Mjc1MmJlZTg3IgogICAgfQogIH0KfQ==";
const FROZEN_ADVENTURER_SKIN_SIGNATURE: &str = "YrsEAf5EY/wDxeAhUAyt2YKybLZ7jDHQLY2wzDoozf43CGHaHinumeqNhq4YT6pHScZWy4xJdFGyhrEWDwoJMRIMDQnQpaTZwMrIdWuMIU3VGd2wMhrkcJvTR1jEmCUr0TUqqVhnLOVQHv0v+YvJw8NQQjwH0wH9WD5zqOyOMEJ5hJ/gdf+vivvXJqFGMTg28Z+CFMe1m0gCEWiynv2LAZ2+NbLQzkYoacNvs5WO0Zo3YnxgN4ps2VnVULq0vSNC6GEGVkuHp+9dTobEwr8apoYGqF6qTChTpwaIT++TiZ6fY8zGXILNK8fegshuM2R9WyN91Jz/6vPR722Gr8Y6KqwDbIAhW/Jb5Q/F6co6Sf4f8nZgg1TqmKf3ntb5dwvCh+zSixA7H5xLf8pb+kxIQcwb2O4qHlgNRHcX1HIwVgBagOAuLUKYesTftzm1dCgpPkqgNxU/l2qZkwyt9j45ExYKuULwEYRsgJvcvufZjfmotfRG88x7ZReVGIl+5HfMd5EoG7QyR+tfOCDEXp1h2FBEOlM/1CPFXU1tYuRBLyvPWCJU/1ystVt+acU8eZjtAl5GqfskQRKGveVWXeTKHZN9H/yxtSXdHzYtp2lZOzLCdOTmoqLu+0YJUq0F3DFn6QYhtAzIyR4KY8fQyq+0Tjsi3hRz0PjHHgPJLZQAqCg=";

/// GameProfile "textures" property value for the Angry Archaeologist NPC body skin (Mojang
/// profile "KAEVERY"), plus its Yggdrasil signature. Sourced the same way as
/// `FROZEN_ADVENTURER_SKIN` above: a local PrismLauncher skin cache file, resolved to the real
/// signed profile via mineskin.org's `/v2/generate` upload endpoint.
const ANGRY_ARCHAEOLOGIST_SKIN: &str = "ewogICJ0aW1lc3RhbXAiIDogMTYxOTk0MzcyNDk3NiwKICAicHJvZmlsZUlkIiA6ICIwMGM2Yjk0YTY5YmU0MzY3OTkwOTQxNjFjMjAxOWI3ZiIsCiAgInByb2ZpbGVOYW1lIiA6ICJLQUVWRVJZIiwKICAic2lnbmF0dXJlUmVxdWlyZWQiIDogdHJ1ZSwKICAidGV4dHVyZXMiIDogewogICAgIlNLSU4iIDogewogICAgICAidXJsIiA6ICJodHRwOi8vdGV4dHVyZXMubWluZWNyYWZ0Lm5ldC90ZXh0dXJlL2M0OGM3ODM0NThlNGNmODUxOGU4YWI1ODYzZmJjNGNiOTQ4ZjkwNTY4ZWViOWE2MGQxNmM0ZmRlMmI5NmMwMzMiCiAgICB9CiAgfQp9";
const ANGRY_ARCHAEOLOGIST_SKIN_SIGNATURE: &str = "rkTKeilcdLpr6W0kMtW2q/sxg3hKMquKUZFlny68mlENRgh7Lv54qkLRXjslGr6ITxBsmvG8yS0pK+DvAW5sYaFqJSowUy0o28uxTBVD1AaWmg4/wzDkS0HUVWFPZgPjUa4o7drZpkwifuTNQqxQkFVwJtSc5KquB0bfsca4QY9X99TIQRHGGg/comjJG+ewGMy8Y2AqN7WJoYOqhKgpdkYEMDL6DIXxWkbe3T1lI2jq77VY8ClIOde9B9VqnDOPoQviF3dXFMgS7a882xVcs0XhxitgM9KKJo+ehCnCdcT2B5RcRR6edvjeqSM++Vn5358G7Fl+R2PqH2LwMM6XjjjZCvujFHZZ3r4u/EN4TUGeZ7xQWm+kVOA1ncPC0ZOhTLFpazLjQFs6jXKSrnsAgr6uyToeRiFJ4AyR0NGoV9tqgBcUIoFSuKOwhEJ489g4VDFCCauQTBVpZloI2kYpRm8knZ5rRpEu8ZxUA5aV0gim3Fft4fMkF7bA2m7TkjZ9jMn2aR0dW6BTU1Rlr3aQeOklnH2kTLI8M/GJ4JOkeWoboHePk2P3gWpQZ43OQ2N92attD1RrgxuiLpoRFvjPoBXdokPJLFvvt84cQf3vTb9AqXFCkh0Yzt8p9U3BzeG6p8bQoH0OCJzRnixK1WJZj8biz4G1MZFB1tggkuOyPHI=";

/// GameProfile "textures" property value for King Midas's NPC body skin (Mojang profile
/// "RawLobsters"), plus its Yggdrasil signature. Sourced the same way as
/// `FROZEN_ADVENTURER_SKIN`/`ANGRY_ARCHAEOLOGIST_SKIN` above.
const KING_MIDAS_SKIN: &str = "ewogICJ0aW1lc3RhbXAiIDogMTYyMjQ3NTc1NDkyMSwKICAicHJvZmlsZUlkIiA6ICJjZGM5MzQ0NDAzODM0ZDdkYmRmOWUyMmVjZmM5MzBiZiIsCiAgInByb2ZpbGVOYW1lIiA6ICJSYXdMb2JzdGVycyIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS82MmJjYTA4NTc1MDA0MzUwM2Y1ZGY5ZjdkZWY4MjRhMmUzYWNmYzI3ODQyYmNkMDlkMmI2Njk1ODgxZTgzMmY1IgogICAgfQogIH0KfQ==";
const KING_MIDAS_SKIN_SIGNATURE: &str = "QnF9T5bPy3ebZybtwl34TuJIpuOyJLBgZKSzCxUEMoJuXeIcPgOSuwTgG6xv8dPNEdU1YwEugZgYVkH+yoUCvf1tY3NT2/8KIlGC1VX4w5D6h4zYLrAJ7CCJRaNDp0SWVXwDXUqQBHR2ZD5bMy+4AtpoAn/rA+FKnsWjfU2E6nU88BrePVPsvE33xMXO9JyG/AAv+yC6/2uIfieCwYYy57ObF+duyaPxH4MhJQ8Mjs1zMj+ZrcMQjaGyTJOx6ReqW4CC+FoeFQdrJFmC53wGezkU4vUWZTrCa/kUSIeTXeVNAcEmU5IRHlXg/sL91iSIIIDp78dgFSU/tYrK+O/rkwbJYYZoMebhFdED1S6ccHIJp+fsuG1nnSmww9F89vqLUy0NdP2+x78Y96Lc29SWNIYIcXPkW3aKQNMYlQ78ugelY541yIWd6jFuNw8JyQGOg+Ixce3D5c9+rZ4sD46b87fLrNczatpo3NEJvqjgaTvyXPKGHltvtTZbjIc6FWJUEw1zSWMcN7E0fAU0ovxRKi+yVMWs7a23vZCcCg/R8abJY8XC/evF03aqJprpcSDQhKy29krQ/wmPNYWPsgMZNH0RTriJOgs9a54Q+pLdVmutVZy2E4TQ3PamBbwst5md7v+nyqLZ8jo0i9Aq5JyAwYgyP1+SjWlRgQfK9hNcvUY=";

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

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
            _ => None,
        }
    }
}

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

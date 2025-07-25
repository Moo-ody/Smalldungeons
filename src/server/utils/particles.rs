crate::particle_enum! {
    ExplosionNormal("explode", 0, true),
    ExplosionLarge("largeexplode", 1, true),
    ExplosionHuge("hugeexplosion", 2, true),
    FireworksSpark("fireworksSpark", 3, false),
    WaterBubble("bubble", 4, false),
    WaterSplash("splash", 5, false),
    WaterWake("wake", 6, false),
    Suspended("suspended", 7, false),
    SuspendedDepth("depthsuspend", 8, false),
    Crit("crit", 9, false),
    CritMagic("magicCrit", 10, false),
    SmokeNormal("smoke", 11, false),
    SmokeLarge("largesmoke", 12, false),
    Spell("spell", 13, false),
    SpellInstant("instantSpell", 14, false),
    SpellMob("mobSpell", 15, false),
    SpellMobAmbient("mobSpellAmbient", 16, false),
    SpellWitch("witchMagic", 17, false),
    DripWater("dripWater", 18, false),
    DripLava("dripLava", 19, false),
    VillagerAngry("angryVillager", 20, false),
    VillagerHappy("happyVillager", 21, false),
    TownAura("townaura", 22, false),
    Note("note", 23, false),
    Portal("portal", 24, false),
    EnchantmentTable("enchantmenttable", 25, false),
    Flame("flame", 26, false),
    Lava("lava", 27, false),
    Footstep("footstep", 28, false),
    Cloud("cloud", 29, false),
    Redstone("reddust", 30, false),
    Snowball("snowballpoof", 31, false),
    SnowShovel("snowshovel", 32, false),
    Slime("slime", 33, false),
    Heart("heart", 34, false),
    Barrier("barrier", 35, false),
    ItemCrack("iconcrack_", 36, false, 2),
    BlockCrack("blockcrack_", 37, false, 1),
    BlockDust("blockdust_", 38, false, 1),
    WaterDrop("droplet", 39, false),
    ItemTake("take", 40, false),
    MobAppearance("mobappearance", 41, true)
}

// impl PacketWrite for ParticleTypes {
//     fn write(&self, buf: &mut Vec<u8>) {
//         self.get_id().write(buf);
//     }
// }

#[macro_export]
macro_rules! particle_enum {
    {$($name:ident($name_str:expr, $id:expr, $ignore_range:expr$(, $arg_count:expr)?)),* $(,)?} => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum ParticleTypes {
            $(
                $name
            ),*
        }
        
        impl ParticleTypes {
            pub const fn from_id(id: i32) -> Option<ParticleTypes> {
                match id {
                    $($id => Some(ParticleTypes::$name),)*
                    _ => None
                }
            }
            
            pub const fn get_id(&self) -> i32 {
                match self {
                    $(ParticleTypes::$name => $id,)*
                }
            }
            
            pub const fn get_name(&self) -> &'static str {
                match self {
                    $(ParticleTypes::$name => $name_str,)*
                }
            }
            
            pub const fn get_arg_count(&self) -> Option<u8> {
                match self {
                    $(
                        ParticleTypes::$name => crate::argcounted!($name$(, $arg_count)*)
                    ),*
                }
            }
            
            pub const fn is_ignore_range(&self) -> bool {
                match self {
                    $(ParticleTypes::$name => $ignore_range),*
                }
            }
        }
    }
}

#[macro_export]
macro_rules! argcounted {
    ($name:ident, $argcount:expr) => {
        Some($argcount)
    };
    ($name:ident) => {
        None
    };
}
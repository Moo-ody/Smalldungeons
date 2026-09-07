use crate::id_enum;

#[macro_export]
macro_rules! id_enum {
    (pub enum $enumName:ident: $idType:ty {$($name:ident ($id:expr)),* $(,)?}) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $enumName {
            $(
                $name,
            )*
        }

        impl $enumName {
            pub const fn id(&self) -> $idType {
                match self {
                    $(
                        $enumName::$name => $id,
                    )*
                }
            }
        }
    }
}

id_enum! {
    pub enum Sounds: &'static str {
        EnderDragonHit("mob.enderdragon.hit"),
        Harp("note.harp"),
        Orb("random.orb"),
        Pop("random.pop"),
        ChestOpen("random.chestopen"),
        Portal("mob.portal"),  
        FireIgnite("fire.ignite"),
        ZombieRemedy("mob.zombie.remedy"),
        RandomClick("random.click"),
        EnderDragonGrowl("mob.enderdragon.growl"),
        VillagerHaggle("mob.villager.haggle"),
        NotePling("note.pling"),
        GhastScream("mob.ghast.scream"),
        GuardianElderHit("mob.guardian.elder.hit"),
        Bow("random.bow"),
        EndermenPortal("mob.endermen.portal"),
        NoteHat("note.hat"),
        RandomExplode("random.explode"),
        GhastMoan("mob.ghast.moan"),
        FireworksBlast("fireworks.blast"),
        FireworksTwinkle("fireworks.twinkle"),
        RandomFizz("random.fizz"),
        RandomWoodClick("random.wood_click"),
        BatDeath("mob.bat.death"),
        BatHurt("mob.bat.hurt"),
        PistonIn("tile.piston.in"),
        PistonOut("tile.piston.out"),
        SkeletonDeath("mob.skeleton.death"),
        EndermenDeath("mob.endermen.death"),
        BlazeDeath("mob.blaze.death"),
        CatMeow("mob.cat.meow"),
        CatPurr("mob.cat.purr"),
        CatPurreow("mob.cat.purreow"),
        DonkeyHit("mob.horse.donkey.hit"),
        LevelUp("random.levelup"),
        // Legacy 1.8 name for modern clients' `block.glass.break`.
        GlassBreak("dig.glass"),
        // Legacy 1.8 name for modern clients' `block.wool.break` - wool was "cloth" in the old
        // flat sound registry.
        ClothBreak("dig.cloth"),
        // Legacy 1.8 name for modern clients' `entity.item.break` (a held item/tool breaking).
        ItemBreak("random.break"),
        // Legacy 1.8 name for modern clients' `entity.player.death`.
        PlayerDeath("game.player.die"),
    }
}

/// A few distinct cat vocalizations to pick from at random - see `Sounds::CatMeow` and friends.
pub const MEOW_SOUNDS: [Sounds; 3] = [Sounds::CatMeow, Sounds::CatPurr, Sounds::CatPurreow];
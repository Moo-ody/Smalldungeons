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
        
    }
}
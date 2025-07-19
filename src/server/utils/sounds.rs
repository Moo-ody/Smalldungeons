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
        EnderDragonHit("mob.enderdragon.hit")
    }
}
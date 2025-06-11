use crate::server::block::block_parameter::Axis;

crate::register_blocks! {
    0 => {
        Air => 0,
    },
    1 => {
        Stone => 0,
        Granite => 1,
        PolishedGranite => 2,
        Diorite => 3,
        PolishedDiorite => 4,
        Andesite => 5,
        PolishedAndesite => 6, // i assume the _ is just in case its a weird number? this shouldnt be possible in vanilla right? adding support for it would be tricky.
                               // it was so it doesn't complain about match statement stuff iirc 
    },
    2 => {
        Grass => 0,
    },
    3 => {
        Dirt => 0,
        CoarseDirt => 1,
        Podzol => 2,
    },
    4 => {
        Cobblestone => 0,
    },
    5 => {
        OakPlanks => 0,
        SprucePlanks => 1,
        BirchPlanks => 2,
        JunglePlanks => 3,
        AcaciaPlanks => 4,
        DarkOakPlanks => 5,
    },
    6 => {
        OakSapling => 0,
        SpruceSapling => 1,
        BirchSapling => 2,
        JungleSapling => 3,
        AcaciaSapling => 4,
        DarkOakSapling => 5,
    },
    7 => {
        Bedrock => 0,
    },
    8 => {
        FlowingWater { level: u8} => level,
    },
    9 => {
        Water => 0,
    },
    10 => {
        FlowingLava { level: u8 } => level,
    },
    11 => {
        Lava => 0,
    },
    12 => {
        Sand => 0,
        RedSand => 1,
    },
    13 => {
        Gravel => 0,
    },
    14 => {
        GoldOre => 0,
    },
    15 => {
        IronOre => 0,
    },
    16 => {
        CoalOre => 0,
    },
    17 => {
        OakWood { axis: Axis } => 0 | ((axis as u8) << 2),
        SpruceWood { axis: Axis } => 1 | ((axis as u8) << 2),
        BirchWood { axis: Axis } => 2 | ((axis as u8) << 2),
        JungleWood { axis: Axis } => 3 | ((axis as u8) << 2),
    },
    35 => {
        WhiteWool => 0,
        OrangeWool => 1,
        MagentaWool => 2,
        LightBlueWool => 3,
        YellowWool => 4,
        LimeWool => 5,
        PinkWool => 6,
        GrayWool => 7,
        LightGrayWool => 8,
        CyanWool => 9,
        PurpleWool => 10,
        BlueWool => 11,
        BrownWool => 12,
        GreenWool => 13,
        RedWool => 14,
        BlackWool => 15,
    }
}

impl Blocks {
    /// write implementation for the [Blocks] enum.
    pub fn from_block_state_id(id: u16) -> Blocks {
        let block_id = id >> 4;
        let meta = (id & 0xF) as u8;
        match block_id {
            0 => Blocks::Air,
            1 => match meta {
                0 => Blocks::Stone,
                1 => Blocks::Granite,
                2 => Blocks::PolishedGranite,
                3 => Blocks::Diorite,
                4 => Blocks::PolishedDiorite,
                5 => Blocks::Andesite,
                _ => Blocks::PolishedAndesite,
            },
            2 => Blocks::Grass,
            3 => match meta {
                0 => Blocks::Dirt,
                1 => Blocks::CoarseDirt,
                _ => Blocks::Podzol,
            },
            4 => Blocks::Cobblestone,
            5 => match meta {
                0 => Blocks::OakPlanks,
                1 => Blocks::SprucePlanks,
                2 => Blocks::BirchPlanks,
                3 => Blocks::JunglePlanks,
                4 => Blocks::AcaciaPlanks,
                _ => Blocks::DarkOakPlanks,
            }
            6 => match meta {
                0 => Blocks::OakSapling,
                1 => Blocks::SpruceSapling,
                2 => Blocks::BirchSapling,
                3 => Blocks::JungleSapling,
                4 => Blocks::AcaciaSapling,
                _ => Blocks::DarkOakSapling,
            }
            7 => Blocks::Bedrock,
            8 => Blocks::FlowingWater { level: meta },
            35 => match meta {
                1 => Blocks::OrangeWool,
                2 => Blocks::MagentaWool,
                3 => Blocks::LightBlueWool,
                4 => Blocks::YellowWool,
                5 => Blocks::LimeWool,
                6 => Blocks::PinkWool,
                7 => Blocks::GrayWool,
                8 => Blocks::LightGrayWool,
                9 => Blocks::CyanWool,
                10 => Blocks::PurpleWool,
                11 => Blocks::BlueWool,
                12 => Blocks::BrownWool,
                13 => Blocks::GreenWool,
                14 => Blocks::RedWool,
                15 => Blocks::BlackWool,
                _ => Blocks::WhiteWool
            }
            _ => todo!() // for now to make it obvious
        }
    }
}

/// macro implementing reading metadata and ids for blocks
///
/// format:
/// ```
/// id => {
///     blockname => metadata (ex: 1),
///     blockname { param1: type, param2: type } => metadata (ex: 0 | param1 << 2)
/// },
/// nextid etc.
/// ```
///
/// the write must be manually written in the [Blocks] enum.
#[macro_export]
macro_rules! register_blocks {
    {
        $($id:expr => {
            $(
                $block:ident $({ $($field:ident: $ty:ty),+ $(,)? })* => $meta:expr
            ),+ $(,)?
        }),+ $(,)?
        //$($block:ident $({ $($field:ident: $ty:ty),+ $(,)?})? = ($id:expr, $meta:expr)),* $(,)?
    } => {
        #[derive(PartialEq, Debug, Copy, Clone)]
        pub enum Blocks {
            $(
                $(
                   $block $( { $($field : $ty),+ } )?,
                )+
            )*
        }
        impl Blocks {
            pub fn block_state_id(self) -> u16 {
                let (id, meta) = match self { 
                    $(
                        $(
                            Blocks::$block $( { $($field),+ } )? => ($id, $meta),
                        )+
                    )*
                };
                (id << 4) | meta as u16
            }
        }
    };
}

#[macro_export]
macro_rules! read_or_meta {
    ($meta:expr, $read:expr) => { $read };
    ($meta:expr) => { $meta };
}
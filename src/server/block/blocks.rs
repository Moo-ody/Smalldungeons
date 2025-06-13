use crate::server::block::block_parameter::{Axis};

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
    18 => {
        OakLeaves { decayable: bool, check_decay: bool } => 0,
        SpruceLeaves { decayable: bool, check_decay: bool } => 1,
        BirchLeaves { decayable: bool, check_decay: bool } => 2,
        JungleLeaves { decayable: bool, check_decay: bool } => 3,
    },
    22 => {
        LapisBlock => 0,
    },
    23 => {
        Dispenser { facing: u8 } => facing,
    },
    31 => {
        TallGrass => 0,
        Fern => 1,
    },
    33 => {
        Piston { facing: u8, extended: bool } => facing + 8 * extended as u8,
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
    },
    38 => {
        Poppy => 0,
        BlueOrchid => 1,
        Allium => 2,
        AzureBluet => 3,
        RedTulip => 4,
        OrangeTulip => 5,
        WhiteTulip => 6,
        PinkTulip => 7,
        OxeyeDaisy => 8,
    },
    42 => {
        IronBlock => 0,
    },
    44 => {
        StoneSlab { top: bool } => 0 + (8 * top as u8),
        SandstoneSlab { top: bool } => 1 + (8 * top as u8),
        AlphaSlab { top: bool } => 2 + (8 * top as u8),
        CobblestoneSlab { top: bool } => 3 + (8 * top as u8),
        BrickSlab { top: bool } => 4 + (8 * top as u8),
        StoneBrickSlab { top: bool } => 5 + (8 * top as u8),
        NetherBrickSlab { top: bool } => 6 + (8 * top as u8),
        QuartzSlab { top: bool } => 7 + (8 * top as u8),
    },
    50 => {
        Torch { facing: u8 } => facing, // TODO: Other facing
    },
    51 => {
        Fire => 0,
    },
    85 => {
        OakFence => 0,
    },
    89 => {
        Glowstone => 0,
    },
    95 => {
        WhiteStainedGlass => 0,
        OrangeStainedGlass => 1,
        MagentaStainedGlass => 2,
        LightBlueStainedGlass => 3,
        YellowStainedGlass => 4,
        LimeStainedGlass => 5,
        PinkStainedGlass => 6,
        GrayStainedGlass => 7,
        LightGrayStainedGlass => 8,
        CyanStainedGlass => 9,
        PurpleStainedGlass => 10,
        BlueStainedGlass => 11,
        BrownStainedGlass => 12,
        GreenStainedGlass => 13,
        RedStainedGlass => 14,
        BlackStainedGlass => 15,
    },
    98 => {
        StoneBrick => 0,
        MossyStoneBrick => 1,
        CrackedStoneBrick => 2,
        ChiselledStoneBrick => 3,
    },
    100 => {
        RedMushroomBlock { variant: u8 } => variant,
    },
    101 => {
        IronBars => 0,
    },
    103 => {
        Melon => 0,
    },
    109 => {
        StoneBrickStairs { facing: u8, half: bool } => facing << half as u8,
    },
    112 => {
        NetherBrick => 0,
    },
    118 => {
        Cauldron => 0,
    },
    126 => {
        OakSlab { top: bool } => 0 + (8 * top as u8),
        SpruceSlab { top: bool } => 1 + (8 * top as u8),
        BirchSlab { top: bool } => 2 + (8 * top as u8),
        JungleSlab { top: bool } => 3 + (8 * top as u8),
        AcaciaSlab { top: bool } => 4 + (8 * top as u8),
        DarkOakSlab { top: bool } => 5 + (8 * top as u8),
    },
    132 => {
        Tripwire { meta: u8 } => meta,
    },
    139 => {
        CobblestoneWall => 0,
        MossyCobblestoneWall => 1,
    },
    140 => {
        FlowerPot => 0,
    },
    144 => {
        Skull { facing: u8, nodrop: bool } => facing << nodrop as u8,
    },
    152 => {
        RedstoneBlock => 0,
    },
    154 => {
        Hopper { facing: u8 } => facing,
    },
    159 => {
        WhiteTerracotta => 0,
        OrangeTerracotta => 1,
        MagentaTerracotta => 2,
        LightBlueTerracotta => 3,
        YellowTerracotta => 4,
        LimeTerracotta => 5,
        PinkTerracotta => 6,
        GrayTerracotta => 7,
        LightGrayTerracotta => 8,
        CyanTerracotta => 9,
        PurpleTerracotta => 10,
        BlueTerracotta => 11,
        BrownTerracotta => 12,
        GreenTerracotta => 13,
        RedTerracotta => 14,
        BlackTerracotta => 15,
    },
    160 => {
        WhiteStainedGlassPane => 0,
        OrangeStainedGlassPane => 1,
        MagentaStainedGlassPane => 2,
        LightBlueStainedGlassPane => 3,
        YellowStainedGlassPane => 4,
        LimeStainedGlassPane => 5,
        PinkStainedGlassPane => 6,
        GrayStainedGlassPane => 7,
        LightGrayStainedGlassPane => 8,
        CyanStainedGlassPane => 9,
        PurpleStainedGlassPane => 10,
        BlueStainedGlassPane => 11,
        BrownStainedGlassPane => 12,
        GreenStainedGlassPane => 13,
        RedStainedGlassPane => 14,
        BlackStainedGlassPane => 15,
    },
    164 => {
        DarkOakStairs { facing: u8, half: bool } => facing << half as u8,
    },
    169 => {
        SeaLantern => 0,
    },
    171 => {
        WhiteCarpet => 0,
        OrangeCarpet => 1,
        MagentaCarpet => 2,
        LightBlueCarpet => 3,
        YellowCarpet => 4,
        LimeCarpet => 5,
        PinkCarpet => 6,
        GrayCarpet => 7,
        LightGrayCarpet => 8,
        CyanCarpet => 9,
        PurpleCarpet => 10,
        BlueCarpet => 11,
        BrownCarpet => 12,
        GreenCarpet => 13,
        RedCarpet => 14,
        BlackCarpet => 15,
    },
    173 => {
        CoalBlock => 0
    },
    188 => {
        SpruceFence => 0,
    },
    189 => {
        BirchFence => 0,
    },
    190 => {
        JungleFence => 0,
    },
    191 => {
        DarkOakFence => 0,
    },
    192 => {
        AcaciaFence => 0,
    },
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
            9 => Blocks::Water,
            10 => Blocks::FlowingLava { level: meta },
            11 => Blocks::Lava,
            12 => match meta {
                0 => Blocks::Sand,
                _ => Blocks::RedSand,
            },
            13 => Blocks::Gravel,
            14 => Blocks::GoldOre,
            15 => Blocks::IronOre,
            16 => Blocks::CoalOre,
            17 => match meta {
                0 => Blocks::OakWood { axis: Axis::Y },
                1 => Blocks::SpruceWood { axis: Axis::Y },
                2 => Blocks::BirchWood { axis: Axis::Y },
                3 => Blocks::JungleWood { axis: Axis::Y },
                4 => Blocks::OakWood { axis: Axis::X },
                5 => Blocks::SpruceWood { axis: Axis::X },
                6 => Blocks::BirchWood { axis: Axis::X },
                7 => Blocks::JungleWood { axis: Axis::X },
                8 => Blocks::OakWood { axis: Axis::Z },
                9 => Blocks::SpruceWood { axis: Axis::Z },
                10 => Blocks::BirchWood { axis: Axis::Z },
                11 => Blocks::JungleWood { axis: Axis::Z },
                12 => Blocks::OakWood { axis: Axis::None },
                13 => Blocks::SpruceWood { axis: Axis::None },
                14 => Blocks::BirchWood { axis: Axis::None },
                _ => Blocks::JungleWood { axis: Axis::None },
            },
            18 => match meta {
                0 => Blocks::OakLeaves { decayable: true, check_decay: false },
                1 => Blocks::SpruceLeaves { decayable: true, check_decay: false },
                2 => Blocks::BirchLeaves { decayable: true, check_decay: false },
                3 => Blocks::JungleLeaves { decayable: true, check_decay: false },
                4 => Blocks::OakLeaves { decayable: false, check_decay: false },
                5 => Blocks::SpruceLeaves { decayable: false, check_decay: false },
                6 => Blocks::BirchLeaves { decayable: false, check_decay: false },
                7 => Blocks::JungleLeaves { decayable: false, check_decay: false },
                8 => Blocks::OakLeaves { decayable: true, check_decay: true },
                9 => Blocks::SpruceLeaves { decayable: true, check_decay: true },
                10 => Blocks::BirchLeaves { decayable: true, check_decay: true },
                11 => Blocks::JungleLeaves { decayable: true, check_decay: true },
                12 => Blocks::OakLeaves { decayable: false, check_decay: true },
                13 => Blocks::SpruceLeaves { decayable: false, check_decay: true },
                14 => Blocks::BirchLeaves { decayable: false, check_decay: true },
                _ => Blocks::JungleLeaves { decayable: false, check_decay: true },
            },
            22 => Blocks::LapisBlock,
            23 => Blocks::Dispenser { facing: meta },
            31 => match meta {
                0 => Blocks::TallGrass,
                _ => Blocks::Fern,
            },
            33 => match meta {
                0 => Blocks::Piston { facing: 0, extended: false },
                1 => Blocks::Piston { facing: 1, extended: false },
                2 => Blocks::Piston { facing: 2, extended: false },
                3 => Blocks::Piston { facing: 3, extended: false },
                4 => Blocks::Piston { facing: 4, extended: false },
                5 => Blocks::Piston { facing: 5, extended: false },
                8 => Blocks::Piston { facing: 0, extended: true },
                9 => Blocks::Piston { facing: 1, extended: true },
                10 => Blocks::Piston { facing: 2, extended: true },
                11 => Blocks::Piston { facing: 3, extended: true },
                12 => Blocks::Piston { facing: 4, extended: true },
                _ => Blocks::Piston { facing: 5, extended: true },
            }
            35 => match meta {
                0 => Blocks::WhiteWool,
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
                _ => Blocks::BlackWool,
            },
            38 => match meta {
                0 => Blocks::Poppy,
                1 => Blocks::BlueOrchid,
                2 => Blocks::Allium,
                3 => Blocks::AzureBluet,
                4 => Blocks::RedTulip,
                5 => Blocks::OrangeTulip,
                6 => Blocks::WhiteTulip,
                7 => Blocks::PinkTulip,
                8 => Blocks::OxeyeDaisy,
                _ => Blocks::Poppy,
            },
            42 => Blocks::IronBlock,
            44 => match meta {
                0 => Blocks::StoneSlab { top: false },
                1 => Blocks::SandstoneSlab { top: false },
                2 => Blocks::AlphaSlab { top: false },
                3 => Blocks::CobblestoneSlab { top: false },
                4 => Blocks::BrickSlab { top: false },
                5 => Blocks::StoneBrickSlab { top: false },
                6 => Blocks::NetherBrickSlab { top: false },
                7 => Blocks::QuartzSlab { top: false },
                8 => Blocks::StoneSlab { top: true },
                9 => Blocks::SandstoneSlab { top: true },
                10 => Blocks::AlphaSlab { top: true },
                11 => Blocks::CobblestoneSlab { top: true },
                12 => Blocks::BrickSlab { top: true },
                13 => Blocks::StoneBrickSlab { top: true },
                14 => Blocks::NetherBrickSlab { top: true },
                _ => Blocks::QuartzSlab { top: true },
            },
            50 => match meta {
                _ => Blocks::Torch { facing: meta },
            },
            51 => Blocks::Fire,
            85 => Blocks::OakFence,
            89 => Blocks::Glowstone,
            95 => match meta {
                0 => Blocks::WhiteStainedGlass,
                1 => Blocks::OrangeStainedGlass,
                2 => Blocks::MagentaStainedGlass,
                3 => Blocks::LightBlueStainedGlass,
                4 => Blocks::YellowStainedGlass,
                5 => Blocks::LimeStainedGlass,
                6 => Blocks::PinkStainedGlass,
                7 => Blocks::GrayStainedGlass,
                8 => Blocks::LightGrayStainedGlass,
                9 => Blocks::CyanStainedGlass,
                10 => Blocks::PurpleStainedGlass,
                11 => Blocks::BlueStainedGlass,
                12 => Blocks::BrownStainedGlass,
                13 => Blocks::GreenStainedGlass,
                14 => Blocks::RedStainedGlass,
                _ => Blocks::BlackStainedGlass,
            },
            98 => match meta {
                0 => Blocks::StoneBrick,
                1 => Blocks::MossyStoneBrick,
                2 => Blocks::CrackedStoneBrick,
                _ => Blocks::ChiselledStoneBrick,
            },
            100 => Blocks::RedMushroomBlock { variant: meta },
            101 => Blocks::IronBars,
            103 => Blocks::Melon,
            109 => match meta {
                0 => Blocks::StoneBrickStairs { facing: 0, half: false },
                1 => Blocks::StoneBrickStairs { facing: 1, half: false },
                2 => Blocks::StoneBrickStairs { facing: 2, half: false },
                3 => Blocks::StoneBrickStairs { facing: 3, half: false },
                4 => Blocks::StoneBrickStairs { facing: 0, half: true },
                5 => Blocks::StoneBrickStairs { facing: 1, half: true },
                6 => Blocks::StoneBrickStairs { facing: 2, half: true },
                _ => Blocks::StoneBrickStairs { facing: 3, half: true },
            },
            112 => Blocks::NetherBrick,
            118 => Blocks::Cauldron,
            126 => match meta {
                0 => Blocks::OakSlab { top: false },
                1 => Blocks::SpruceSlab { top: false },
                2 => Blocks::BirchSlab { top: false },
                3 => Blocks::JungleSlab { top: false },
                4 => Blocks::AcaciaSlab { top: false },
                5 => Blocks::DarkOakSlab { top: false },
                8 => Blocks::OakSlab { top: true },
                9 => Blocks::SpruceSlab { top: true },
                10 => Blocks::BirchSlab { top: true },
                11 => Blocks::JungleSlab { top: true },
                12 => Blocks::AcaciaSlab { top: true },
                _ => Blocks::DarkOakSlab { top: true },
            },
            132 => Blocks::Tripwire { meta: meta },
            139 => match meta {
                0 => Blocks::CobblestoneWall,
                _ => Blocks::MossyCobblestoneWall,
            },
            140 => Blocks::FlowerPot,
            144 => match meta {
                0 => Blocks::Skull { facing: 0, nodrop: false },
                1 => Blocks::Skull { facing: 1, nodrop: false },
                2 => Blocks::Skull { facing: 2, nodrop: false },
                3 => Blocks::Skull { facing: 3, nodrop: false },
                4 => Blocks::Skull { facing: 4, nodrop: false },
                5 => Blocks::Skull { facing: 5, nodrop: false },
                6 => Blocks::Skull { facing: 6, nodrop: false },
                7 => Blocks::Skull { facing: 7, nodrop: false },
                8 => Blocks::Skull { facing: 0, nodrop: true },
                9 => Blocks::Skull { facing: 1, nodrop: true },
                10 => Blocks::Skull { facing: 2, nodrop: true },
                11 => Blocks::Skull { facing: 3, nodrop: true },
                12 => Blocks::Skull { facing: 4, nodrop: true },
                13 => Blocks::Skull { facing: 5, nodrop: true },
                14 => Blocks::Skull { facing: 6, nodrop: true },
                _ => Blocks::Skull { facing: 7, nodrop: true },
            },
            152 => Blocks::RedstoneBlock,
            154 => Blocks::Hopper { facing: meta },
            159 => match meta {
                0 => Blocks::WhiteTerracotta,
                1 => Blocks::OrangeTerracotta,
                2 => Blocks::MagentaTerracotta,
                3 => Blocks::LightBlueTerracotta,
                4 => Blocks::YellowTerracotta,
                5 => Blocks::LimeTerracotta,
                6 => Blocks::PinkTerracotta,
                7 => Blocks::GrayTerracotta,
                8 => Blocks::LightGrayTerracotta,
                9 => Blocks::CyanTerracotta,
                10 => Blocks::PurpleTerracotta,
                11 => Blocks::BlueTerracotta,
                12 => Blocks::BrownTerracotta,
                13 => Blocks::GreenTerracotta,
                14 => Blocks::RedTerracotta,
                _ => Blocks::BlackTerracotta,
            },
            160 => match meta {
                0 => Blocks::WhiteStainedGlassPane,
                1 => Blocks::OrangeStainedGlassPane,
                2 => Blocks::MagentaStainedGlassPane,
                3 => Blocks::LightBlueStainedGlassPane,
                4 => Blocks::YellowStainedGlassPane,
                5 => Blocks::LimeStainedGlassPane,
                6 => Blocks::PinkStainedGlassPane,
                7 => Blocks::GrayStainedGlassPane,
                8 => Blocks::LightGrayStainedGlassPane,
                9 => Blocks::CyanStainedGlassPane,
                10 => Blocks::PurpleStainedGlassPane,
                11 => Blocks::BlueStainedGlassPane,
                12 => Blocks::BrownStainedGlassPane,
                13 => Blocks::GreenStainedGlassPane,
                14 => Blocks::RedStainedGlassPane,
                _ => Blocks::BlackStainedGlassPane,
            },
            164 => match meta {
                0 => Blocks::DarkOakStairs { facing: 0, half: false },
                1 => Blocks::DarkOakStairs { facing: 1, half: false },
                2 => Blocks::DarkOakStairs { facing: 2, half: false },
                3 => Blocks::DarkOakStairs { facing: 3, half: false },
                4 => Blocks::DarkOakStairs { facing: 0, half: true },
                5 => Blocks::DarkOakStairs { facing: 1, half: true },
                6 => Blocks::DarkOakStairs { facing: 2, half: true },
                _ => Blocks::DarkOakStairs { facing: 3, half: true },
            },
            169 => Blocks::SeaLantern,
            171 => match meta {
                0 => Blocks::WhiteCarpet,
                1 => Blocks::OrangeCarpet,
                2 => Blocks::MagentaCarpet,
                3 => Blocks::LightBlueCarpet,
                4 => Blocks::YellowCarpet,
                5 => Blocks::LimeCarpet,
                6 => Blocks::PinkCarpet,
                7 => Blocks::GrayCarpet,
                8 => Blocks::LightGrayCarpet,
                9 => Blocks::CyanCarpet,
                10 => Blocks::PurpleCarpet,
                11 => Blocks::BlueCarpet,
                12 => Blocks::BrownCarpet,
                13 => Blocks::GreenCarpet,
                14 => Blocks::RedCarpet,
                _ => Blocks::BlackCarpet,
            },
            173 => Blocks::CoalBlock,
            188 => Blocks::SpruceFence,
            189 => Blocks::BirchFence,
            190 => Blocks::JungleFence,
            191 => Blocks::DarkOakFence,
            192 => Blocks::AcaciaFence,
            _ => Blocks::LimeWool // for now to make it obvious
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
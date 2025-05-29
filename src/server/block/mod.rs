pub mod block_parameter;
pub mod blocks;


// enum moved to [blocks]
// #[derive(PartialEq)]
// pub enum Blocks {
//     Air,
//     Stone,
//     Granite,
//     PolishedGranite,
//     Diorite,
//     PolishedDiorite,
//     Andesite,
//     PolishedAndesite,
//     Grass,
//     Dirt,
//     CoarseDirt,
//     Podzol,
//     Cobblestone,
//     OakPlanks,
//     SprucePlanks,
//     BirchPlanks,
//     JunglePlanks,
//     AcaciaPlanks,
//     DarkOakPlanks,
//     OakSapling,
//     SpruceSapling,
//     BirchSapling,
//     JungleSapling,
//     AcaciaSapling,
//     DarkOakSapling,
//     Bedrock,
//     FlowingWater {
//         level: u8,
//     },
//     Water,
//     FlowingLava {
//         level: u8,
//     },
//     Lava,
//     Sand,
//     RedSand,
//     Gravel,
//     GoldOre,
//     IronOre,
//     CoalOre,
//     OakWood {
//         axis: Axis,
//     },
//     SpruceWood {
//         axis: Axis
//     },
//     BirchWood {
//         axis: Axis
//     },
//     JungleWood {
//         axis: Axis
//     },
//     OakLeaves {
//         check_decay: bool,
//         decayable: bool,
//     },
//     SpruceLeaves {
//         check_decay: bool,
//         decayable: bool,
//     },
//     BirchLeaves {
//         check_decay: bool,
//         decayable: bool,
//     },
//     JungleLeaves {
//         check_decay: bool,
//         decayable: bool,
//     },
//     // etc
// }
// 
// // todo: need some reasonable block id + metadata method, in minecraft it is a hashmap
// 
// impl Blocks {
//     pub fn block_state_id(self) -> u16 {
//         let (id, meta) = match self {
//             Blocks::Air => (0, 0),
//             Blocks::Stone => (1, 0),
//             Blocks::Granite => (1, 1),
//             Blocks::PolishedGranite => (1, 2),
//             Blocks::Diorite => (1, 3),
//             Blocks::PolishedDiorite => (1, 4),
//             Blocks::Andesite => (1, 5),
//             Blocks::PolishedAndesite => (1, 6),
//             Blocks::Grass => (2, 0),
//             Blocks::Dirt => (3, 0),
//             Blocks::CoarseDirt => (3, 1),
//             Blocks::Podzol => (3, 2),
//             Blocks::Cobblestone => (4, 0),
//             Blocks::OakPlanks => (5, 0),
//             Blocks::SprucePlanks => (5, 1),
//             Blocks::BirchPlanks => (5, 2),
//             Blocks::JunglePlanks => (5, 3),
//             Blocks::AcaciaPlanks => (5, 4),
//             Blocks::DarkOakPlanks => (5, 5),
//             Blocks::OakSapling => (6, 0),
//             Blocks::SpruceSapling => (6, 1),
//             Blocks::BirchSapling => (6, 2),
//             Blocks::JungleSapling => (6, 3),
//             Blocks::AcaciaSapling => (6, 4),
//             Blocks::DarkOakSapling => (6, 5),
//             Blocks::Bedrock => (7, 0),
//             Blocks::FlowingWater { level } => (8, level),
//             Blocks::Water => (9, 0),
//             Blocks::FlowingLava { level } => (10, level),
//             Blocks::Lava => (11, 0),
//             Blocks::Sand => (12, 0),
//             Blocks::RedSand => (12, 1),
//             Blocks::Gravel => (13, 0),
//             Blocks::GoldOre => (14, 0),
//             Blocks::IronOre => (15, 0),
//             Blocks::CoalOre => (16, 0),
//             Blocks::OakWood { axis } => (17, 0 | ((axis as u8) << 2)),
//             Blocks::SpruceWood { axis } => (17, 1 | ((axis as u8) << 2)),
//             Blocks::BirchWood { axis } => (17, 3 | ((axis as u8) << 2)),
//             Blocks::JungleWood { axis } => (17, 3 | ((axis as u8) << 2)),
//             Blocks::OakLeaves { check_decay, decayable } => {
//                 (18, ((check_decay as u8) << 3) | ((!decayable as u8) << 2) | 0)
//             },
//             Blocks::SpruceLeaves { check_decay, decayable } => {
//                 (18, ((check_decay as u8) << 3) | ((!decayable as u8) << 2) | 1)
//             }
//             Blocks::BirchLeaves { check_decay, decayable } => {
//                 (18, ((check_decay as u8) << 3) | ((!decayable as u8) << 2) | 2)
//             }
//             Blocks::JungleLeaves { check_decay, decayable } => {
//                 (18, ((check_decay as u8) << 3) | ((!decayable as u8) << 2) | 3)
//             }
//         };
//         (id << 4) | meta as u16
//     }
// 
//     pub fn from_block_state_id(id: u16) -> Blocks {
//         let block_id = id >> 4;
//         let meta = (id & 0xF) as u8;
//         match block_id {
//             0 => Blocks::Air,
//             1 => match meta {
//                 0 => Blocks::Stone,
//                 1 => Blocks::Granite,
//                 2 => Blocks::PolishedGranite,
//                 3 => Blocks::Diorite,
//                 4 => Blocks::PolishedDiorite,
//                 5 => Blocks::Andesite,
//                 _ => Blocks::PolishedAndesite,
//             },
//             2 => Blocks::Grass,
//             3 => match meta {
//                 0 => Blocks::Dirt,
//                 1 => Blocks::CoarseDirt,
//                 _ => Blocks::Podzol,
//             },
//             4 => Blocks::Cobblestone,
//             5 => match meta {
//                 0 => Blocks::OakPlanks,
//                 1 => Blocks::SprucePlanks,
//                 2 => Blocks::BirchPlanks,
//                 3 => Blocks::JunglePlanks,
//                 4 => Blocks::AcaciaPlanks,
//                 _ => Blocks::DarkOakPlanks,
//             }
//             6 => match meta {
//                 0 => Blocks::OakSapling,
//                 1 => Blocks::SpruceSapling,
//                 2 => Blocks::BirchSapling,
//                 3 => Blocks::JungleSapling,
//                 4 => Blocks::AcaciaSapling,
//                 _ => Blocks::DarkOakSapling,
//             }
//             7 => Blocks::Bedrock,
//             8 => Blocks::FlowingWater { level: meta },
//             _ => todo!() // for now to make it obvious
//         }
//     }
// }
// use crate::server::entity::armor_stand::{ArmorStand, ArmorStandImpl};
// use crate::server::entity::entity::{Entity, EntityImpl};
// use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
// use crate::server::utils::dvec3::DVec3;
// use crate::server::block::block_position::BlockPos;
// use crate::server::player::player::Player;
// use crate::server::world::World;
// use std::collections::HashMap;

// #[derive(Debug, Clone, PartialEq)]
// pub enum TerminalType {
//     Terminal,
//     Lever,
//     SimonSays,
//     Lamps,
//     Align,
//     ShootTarget,
// }

// #[derive(Debug, Clone)]
// pub struct Terminal {
//     pub id: u32,
//     pub terminal_type: TerminalType,
//     pub position: BlockPos,
//     pub completed: bool,
//     pub armor_stands: Vec<u32>, // Entity IDs of armor stands
//     pub target_position: Option<BlockPos>, // For lever terminals
// }

// impl Terminal {
//     pub fn new(id: u32, terminal_type: TerminalType, position: BlockPos) -> Self {
//         Self {
//             id,
//             terminal_type,
//             position,
//             completed: false,
//             armor_stands: Vec::new(),
//             target_position: None,
//         }
//     }

//     pub fn with_target(mut self, target: BlockPos) -> Self {
//         self.target_position = Some(target);
//         self
//     }

//     pub fn spawn_armor_stands(&mut self, world: &mut World) {
//         let entity_id = world.get_next_entity_id();
        
//         // Create main armor stand
//         let main_stand = ArmorStand::new(entity_id, self.position.as_dvec3())
//             .with_terminal(self.id)
//             .with_name(self.get_display_name());
        
//         // Create entity and entity impl
//         let mut entity = Entity::new(world, entity_id as i32, self.position.as_dvec3(), EntityMetadata::new(EntityVariant::ArmorStand));
//         let armor_stand_impl = ArmorStandImpl::new(main_stand);
        
//         // Add to world
//         world.entities.insert(entity_id as i32, (entity, Box::new(armor_stand_impl)));
//         self.armor_stands.push(entity_id);
        
//         // Create secondary armor stand for "CLICK HERE" text if it's a terminal
//         if self.terminal_type == TerminalType::Terminal {
//             let secondary_entity_id = world.get_next_entity_id();
//             let secondary_pos = self.position.as_dvec3() + DVec3::new(0.0, -0.35, 0.0);
            
//             let secondary_stand = ArmorStand::new(secondary_entity_id, secondary_pos)
//                 .with_terminal(self.id)
//                 .with_name("§e§lCLICK HERE".to_string());
            
//             let mut secondary_entity = Entity::new(world, secondary_entity_id as i32, secondary_pos, EntityMetadata::new(EntityVariant::ArmorStand));
//             let secondary_armor_stand_impl = ArmorStandImpl::new(secondary_stand);
            
//             world.entities.insert(secondary_entity_id as i32, (secondary_entity, Box::new(secondary_armor_stand_impl)));
//             self.armor_stands.push(secondary_entity_id);
//         }
//     }

//     pub fn destroy_armor_stands(&self, world: &mut World) {
//         for &entity_id in &self.armor_stands {
//             world.destroy_entity(entity_id);
//         }
//     }

//     pub fn get_display_name(&self) -> String {
//         match self.terminal_type {
//             TerminalType::Terminal => "§cInactive Terminal".to_string(),
//             TerminalType::Lever => "§cNot Activated".to_string(),
//             TerminalType::SimonSays => "§cSimon Says Device".to_string(),
//             TerminalType::Lamps => "§cLamps Device".to_string(),
//             TerminalType::Align => "§cAlign Device".to_string(),
//             TerminalType::ShootTarget => "§cShoot Target Device".to_string(),
//         }
//     }

//     pub fn open(&self, player: &mut Player) {
//         if self.completed {
//             return;
//         }
        
//         match self.terminal_type {
//             TerminalType::Terminal => {
//                 // TODO: Open terminal GUI
//                 println!("Opening terminal GUI for terminal {}", self.id);
//             }
//             TerminalType::SimonSays => {
//                 // TODO: Open Simon Says puzzle
//             }
//             _ => {
//                 println!("Terminal type {:?} not yet implemented", self.terminal_type);
//             }
//         }
//     }

//     pub fn complete(&mut self) {
//         self.completed = true;
//         // TODO: Update armor stand display name to show completed state
//     }
// }

// pub struct TerminalManager {
//     terminals: HashMap<u32, Terminal>,
//     next_terminal_id: u32,
//     next_entity_id: u32,
// }

// impl TerminalManager {
//     pub fn new() -> Self {
//         Self {
//             terminals: HashMap::new(),
//             next_terminal_id: 1,
//             next_entity_id: 1000, // Start entity IDs at 1000 to avoid conflicts
//         }
//     }

//     pub fn create_terminal(&mut self, terminal_type: TerminalType, position: BlockPos) -> u32 {
//         let id = self.next_terminal_id;
//         self.next_terminal_id += 1;
        
//         let terminal = Terminal::new(id, terminal_type, position);
//         self.terminals.insert(id, terminal);
//         id
//     }

//     pub fn create_lever_terminal(&mut self, position: BlockPos, target: BlockPos) -> u32 {
//         let id = self.next_terminal_id;
//         self.next_terminal_id += 1;
        
//         let terminal = Terminal::new(id, TerminalType::Lever, position)
//             .with_target(target);
//         self.terminals.insert(id, terminal);
//         id
//     }

//     pub fn get_terminal(&self, id: u32) -> Option<&Terminal> {
//         self.terminals.get(&id)
//     }

//     pub fn get_terminal_mut(&mut self, id: u32) -> Option<&mut Terminal> {
//         self.terminals.get_mut(&id)
//     }

//     pub fn get_terminal_by_entity_id(&self, entity_id: u32) -> Option<&Terminal> {
//         for terminal in self.terminals.values() {
//             if terminal.armor_stands.contains(&entity_id) {
//                 return Some(terminal);
//             }
//         }
//         None
//     }

//     pub fn get_terminal_by_entity_id_mut(&mut self, entity_id: u32) -> Option<&mut Terminal> {
//         for terminal in self.terminals.values_mut() {
//             if terminal.armor_stands.contains(&entity_id) {
//                 return Some(terminal);
//             }
//         }
//         None
//     }

//     pub fn spawn_all_terminals(&mut self, world: &mut World) {
//         for terminal in self.terminals.values_mut() {
//             terminal.spawn_armor_stands(world);
//         }
//     }

//     pub fn destroy_all_terminals(&self, world: &mut World) {
//         for terminal in self.terminals.values() {
//             terminal.destroy_armor_stands(world);
//         }
//     }

//     pub fn get_next_entity_id(&mut self) -> u32 {
//         let id = self.next_entity_id;
//         self.next_entity_id += 1;
//         id
//     }

//     pub fn get_completed_count(&self) -> usize {
//         self.terminals.values().filter(|t| t.completed).count()
//     }

//     pub fn get_total_count(&self) -> usize {
//         self.terminals.len()
//     }

//     pub fn get_terminals_by_type(&self, terminal_type: TerminalType) -> Vec<&Terminal> {
//         self.terminals.values()
//             .filter(|t| t.terminal_type == terminal_type)
//             .collect()
//     }

//     pub fn clear(&mut self) {
//         self.terminals.clear();
//         self.next_terminal_id = 1;
//         self.next_entity_id = 1000;
//     }
// }

// impl Default for TerminalManager {
//     fn default() -> Self {
//         Self::new()
//     }
// }

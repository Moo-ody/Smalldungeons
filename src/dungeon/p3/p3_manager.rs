use crate::server::world::World;
use crate::server::player::player::Player;
use crate::server::block::block_position::BlockPos;
use crate::server::utils::dvec3::DVec3;
use crate::dungeon::p3::terminal::{Terminal, TerminalType, TerminalManager};
use crate::dungeon::p3::simon_says::SimonSays;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Section {
    S1,
    S2,
    S3,
    S4,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
}

pub struct DungeonGate {
    pub position: BlockPos,
    pub rotation: i32,
    pub section: Section,
    pub placed: bool,
}

impl DungeonGate {
    pub fn new(position: BlockPos, rotation: i32, section: Section) -> Self {
        Self {
            position,
            rotation,
            section,
            placed: false,
        }
    }

    pub fn place(&mut self, world: &mut World) {
        // TODO: Implement gate placement logic
        self.placed = true;
        println!("Placed gate at {:?} for section {:?}", self.position, self.section);
    }

    pub fn section_over(&mut self, world: &mut World) {
        // TODO: Implement gate removal logic
        self.placed = false;
        println!("Removed gate at {:?} for section {:?}", self.position, self.section);
    }
}

pub struct P3Manager {
    pub started: bool,
    pub started_time: u64,
    pub current_section: Section,
    pub current_phase: Phase,
    pub terminals: Vec<Terminal>,
    pub devices: Vec<Terminal>,
    pub has_melody: bool,
    pub no_melody: bool,
    pub s1_gate: Option<DungeonGate>,
    pub s2_gate: Option<DungeonGate>,
    pub s3_gate: Option<DungeonGate>,
    pub terminal_manager: TerminalManager,
}

impl P3Manager {
    pub fn new() -> Self {
        Self {
            started: false,
            started_time: 0,
            current_section: Section::S1,
            current_phase: Phase::P3,
            terminals: Vec::new(),
            devices: Vec::new(),
            has_melody: false,
            no_melody: false,
            s1_gate: None,
            s2_gate: None,
            s3_gate: None,
            terminal_manager: TerminalManager::new(),
        }
    }

    pub fn start(&mut self) {
        self.start_section(Section::S1);
    }

    pub fn start_section(&mut self, section: Section) {
        self.current_phase = Phase::P3;
        self.current_section = section;
        
        // TODO: Get client config for no_melody setting
        self.no_melody = false;
        
        self.started = true;
        
        // Get terminals and devices for this section
        self.terminals = self.get_terminal_locations();
        self.devices = self.get_device_locations();
    }
    
    pub fn get_terminals_to_spawn(&self) -> Vec<Terminal> {
        self.terminals.clone()
    }
    
    pub fn get_devices_to_spawn(&self) -> Vec<Terminal> {
        self.devices.clone()
    }

    pub fn get_boss_message() -> String {
        "[BOSS] Goldor: Who dares trespass into my domain?".to_string()
    }

    pub fn get_terminals(&self) -> &Vec<Terminal> {
        &self.terminals
    }

    pub fn get_devices(&self) -> &Vec<Terminal> {
        &self.devices
    }

    fn get_terminal_locations(&self) -> Vec<Terminal> {
        match self.current_section {
            Section::S1 => {
                vec![
                    Terminal::new(1, TerminalType::Terminal, BlockPos::new(110, 112, 73)),
                    Terminal::new(2, TerminalType::Terminal, BlockPos::new(110, 118, 79)),
                    Terminal::new(3, TerminalType::Terminal, BlockPos::new(90, 111, 92)),
                    Terminal::new(4, TerminalType::Terminal, BlockPos::new(90, 121, 101)),
                    Terminal::new(5, TerminalType::Lever, BlockPos::new(94, 122, 113)).with_target(BlockPos::new(94, 124, 113)),
                    Terminal::new(6, TerminalType::Lever, BlockPos::new(106, 122, 113)).with_target(BlockPos::new(106, 124, 113)),
                ]
            }
            Section::S2 => {
                vec![
                    Terminal::new(7, TerminalType::Terminal, BlockPos::new(68, 108, 122)),
                    Terminal::new(8, TerminalType::Terminal, BlockPos::new(59, 119, 123)),
                    Terminal::new(9, TerminalType::Terminal, BlockPos::new(47, 108, 122)),
                    Terminal::new(10, TerminalType::Terminal, BlockPos::new(39, 107, 142)),
                    Terminal::new(11, TerminalType::Terminal, BlockPos::new(40, 123, 123)),
                    Terminal::new(12, TerminalType::Lever, BlockPos::new(27, 122, 127)).with_target(BlockPos::new(27, 124, 127)),
                    Terminal::new(13, TerminalType::Lever, BlockPos::new(23, 130, 138)).with_target(BlockPos::new(23, 132, 138)),
                ]
            }
            Section::S3 => {
                vec![
                    Terminal::new(14, TerminalType::Terminal, BlockPos::new(-2, 108, 112)),
                    Terminal::new(15, TerminalType::Terminal, BlockPos::new(-2, 118, 93)),
                    Terminal::new(16, TerminalType::Terminal, BlockPos::new(18, 122, 93)),
                    Terminal::new(17, TerminalType::Terminal, BlockPos::new(-2, 108, 77)),
                    Terminal::new(18, TerminalType::Lever, BlockPos::new(14, 120, 55)).with_target(BlockPos::new(14, 122, 55)),
                    Terminal::new(19, TerminalType::Lever, BlockPos::new(2, 120, 55)).with_target(BlockPos::new(2, 122, 55)),
                ]
            }
            Section::S4 => {
                vec![
                    Terminal::new(20, TerminalType::Terminal, BlockPos::new(41, 108, 30)),
                    Terminal::new(21, TerminalType::Terminal, BlockPos::new(44, 120, 30)),
                    Terminal::new(22, TerminalType::Terminal, BlockPos::new(67, 108, 30)),
                    Terminal::new(23, TerminalType::Terminal, BlockPos::new(72, 114, 47)),
                    Terminal::new(24, TerminalType::Lever, BlockPos::new(84, 119, 34)).with_target(BlockPos::new(84, 121, 34)),
                    Terminal::new(25, TerminalType::Lever, BlockPos::new(86, 126, 46)).with_target(BlockPos::new(86, 128, 46)),
                ]
            }
        }
    }

    fn get_device_locations(&self) -> Vec<Terminal> {
        // Device locations are the same regardless of section - they're all placed at once
        vec![
            Terminal::new(100, TerminalType::SimonSays, BlockPos::new(110, 118, 91)), // S1 Simon Says
            Terminal::new(101, TerminalType::Lamps, BlockPos::new(60, 130, 142)),     // S2 Lamps
            Terminal::new(102, TerminalType::Align, BlockPos::new(-2, 118, 74)),      // S3 Align
            Terminal::new(103, TerminalType::ShootTarget, BlockPos::new(63, 125, 34)), // S4 Shoot Target
        ]
    }

    pub fn get_terminal_display_name(&self, terminal_type: &TerminalType) -> String {
        match terminal_type {
            TerminalType::Terminal => "§cInactive Terminal".to_string(),
            TerminalType::Lever => "§cNot Activated".to_string(),
            TerminalType::SimonSays => "§cSimon Says Device".to_string(),
            TerminalType::Lamps => "§cLamps Device".to_string(),
            TerminalType::Align => "§cAlign Device".to_string(),
            TerminalType::ShootTarget => "§cShoot Target Device".to_string(),
        }
    }

    pub fn get_terminal_down_name(&self, terminal_type: &TerminalType) -> Option<String> {
        match terminal_type {
            TerminalType::Terminal => Some("§e§lCLICK HERE".to_string()),
            TerminalType::Lever => None,
            _ => None,
        }
    }


    pub fn terminal_completed(&mut self, progress: (usize, usize)) {
        if progress.0 == progress.1 {
            self.next_section();
        }
    }

    pub fn next_section(&mut self) {
        self.has_melody = false;
        
        match self.current_section {
            Section::S1 => {
                self.current_section = Section::S2;
            }
            Section::S2 => {
                self.current_section = Section::S3;
            }
            Section::S3 => {
                self.current_section = Section::S4;
            }
            Section::S4 => {
                self.reset_terminals();
                return;
            }
        }
    }

    pub fn get_term_progress(&self) -> (usize, usize) {
        let mut completed_count = 0;
        
        for terminal in &self.terminals {
            if terminal.completed {
                completed_count += 1;
            }
        }
        
        // Add device completion
        let device = self.get_device_for_section();
        if let Some(device) = device {
            if device.completed {
                completed_count += 1;
            }
        }
        
        (completed_count, self.terminals.len() + 1)
    }

    fn get_device_for_section(&self) -> Option<&Terminal> {
        match self.current_section {
            Section::S1 => self.get_device(TerminalType::SimonSays),
            Section::S2 => self.get_device(TerminalType::Lamps),
            Section::S3 => self.get_device(TerminalType::Align),
            Section::S4 => self.get_device(TerminalType::ShootTarget),
        }
    }

    pub fn get_device(&self, device_type: TerminalType) -> Option<&Terminal> {
        self.devices.iter().find(|device| device.terminal_type == device_type)
    }

    pub fn reset_terminals(&mut self) {
        self.terminals.clear();
        self.devices.clear();
    }
}

impl Default for P3Manager {
    fn default() -> Self {
        Self::new()
    }
}

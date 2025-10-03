use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::metadata::BlockMetadata;
use crate::server::player::player::ClientId;
use crate::server::world::World;
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimonSaysButton {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SimonSaysAction {
    BlockClick,
    ShowSolution,
    Continue,
    Fail,
}

impl SimonSaysButton {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn to_block_pos(&self) -> BlockPos {
        BlockPos::new(self.x, self.y, self.z)
    }
}

pub struct SimonSays {
    pub completed: bool,
    start_clicks: u32,
    progress: usize,
    solution: Vec<SimonSaysButton>,
    start_time: Option<u64>,
    last_clicked: Option<u64>,
    pub showing_solution: bool,
    is_skip: bool,
    clicked_this_tick: bool,
    rng: rand::rngs::ThreadRng,
}

// Constants from Java code
const BOT_LEFT: SimonSaysButton = SimonSaysButton { x: 110, y: 120, z: 92 };
const START_BUTTON: SimonSaysButton = SimonSaysButton { x: 110, y: 121, z: 91 };

impl SimonSays {
    pub fn new() -> Self {
        Self {
            completed: false,
            start_clicks: 0,
            progress: 0,
            solution: Vec::new(),
            start_time: None,
            last_clicked: None,
            showing_solution: false,
            is_skip: false,
            clicked_this_tick: false,
            rng: rand::rng(),
        }
    }

    pub fn should_run(&self) -> bool {
        // TODO: Check if we're in P3 phase
        !self.completed
    }

    pub fn handle_button_click(&mut self, pos: BlockPos, player_id: ClientId, current_tick: u64) -> Option<SimonSaysAction> {
        if !self.should_run() {
            return None;
        }

        if self.showing_solution {
            return Some(SimonSaysAction::BlockClick); // Block clicks during solution display
        }

        if self.is_start_button(pos) {
            self.handle_start_button_click(current_tick);
            return Some(SimonSaysAction::ShowSolution);
        } else if !self.solution.is_empty() && self.is_solution_button(pos) {
            self.handle_solution_button_click(player_id, current_tick);
            return Some(SimonSaysAction::Continue);
        } else if !self.solution.is_empty() {
            self.fail(player_id);
            return Some(SimonSaysAction::Fail);
        }

        None
    }

    fn is_start_button(&self, pos: BlockPos) -> bool {
        pos == START_BUTTON.to_block_pos()
    }

    fn is_solution_button(&self, pos: BlockPos) -> bool {
        if self.progress >= self.solution.len() {
            return false;
        }

        let expected_pos = self.solution[self.progress].to_block_pos();
        let actual_pos = BOT_LEFT.to_block_pos().add(expected_pos);
        
        pos == actual_pos
    }

    fn handle_start_button_click(&mut self, current_tick: u64) {
        let current_time = current_tick * 50; // Convert ticks to milliseconds (50ms per tick)

        // Check cooldown (300ms in Java)
        if let Some(last_clicked) = self.last_clicked {
            if current_time - last_clicked < 300 {
                return;
            }
        }

        if self.start_clicks == 0 {
            self.start_time = Some(current_time);
        }

        self.clicked_this_tick = true;
        self.start_clicks += 1;
        self.last_clicked = Some(current_time);
        self.progress = 0;

        match self.start_clicks {
            3 => {
                // Single skip
                self.is_skip = true;
                self.fill_solution(3);
            }
            7 => {
                // Double
                self.fill_solution(3);
            }
            15 => {
                // Triple
                self.fill_solution(4);
            }
            18 | 21 => {
                // Quad
                self.fill_solution(5);
            }
            24 | 58 => {
                // I1 - complete puzzle
                self.reset(true);
                self.completed = true;
                // TODO: Mark terminal as completed
            }
            _ => {
                // Default
                self.fill_solution(2);
            }
        }
    }

    fn handle_solution_button_click(&mut self, player_id: ClientId, current_tick: u64) {
        self.clicked_this_tick = true;
        self.progress += 1;

        // TODO: Play success sound
        // Utils.sendPacket(player, new S29PacketSoundEffect("note.pling", ...));

        if self.progress >= 5 {
            // Puzzle completed
            if let Some(start_time) = self.start_time {
                let total_time = (current_tick * 50 - start_time) as f64 / 1000.0;
                // TODO: Send completion message
            }
            self.reset(true);
            self.completed = true;
            // TODO: Mark terminal as completed
        } else if self.progress >= self.solution.len() {
            // Sequence completed, add next button
            self.progress = 0;
            self.add_solution();
            // TODO: Show solution
        }
    }

    fn fail(&mut self, _player_id: ClientId) {
        self.reset(false);
        // TODO: Play fail sound and send message
        // Utils.playSoundAll("note.pling", 1f, 0.1f);
        // ChatUtils.sendMessageToAllPlayers(format!("{} failed SS! ♿", player_name));
    }

    pub fn show_solution(&mut self, world: &mut World) {
        if self.showing_solution || self.solution.is_empty() {
            return;
        }

        self.showing_solution = true;
        self.remove_buttons(world);
        
        // TODO: Implement visual feedback with sea lanterns
        // For now, just set a flag that we're showing the solution
    }

    pub fn remove_buttons(&self, world: &mut World) {
        for y in 0..4 {
            for z in 0..4 {
                let pos = BOT_LEFT.to_block_pos().add(BlockPos::new(0, y, z));
                world.set_block_at(Blocks::Air, pos.x, pos.y, pos.z);
            }
        }
    }

    pub fn replace_buttons(&self, world: &mut World) {
        for y in 0..4 {
            for z in 0..4 {
                let pos = BOT_LEFT.to_block_pos().add(BlockPos::new(0, y, z));
                world.set_block_at(Blocks::StoneButton { direction: crate::server::block::block_parameter::ButtonDirection::from_meta(4), powered: false }, pos.x, pos.y, pos.z);
            }
        }
    }

    fn fill_solution(&mut self, count: usize) {
        self.solution.clear();
        for _ in 0..count {
            if self.solution.len() > 9 {
                return;
            }
            if let Some(pos) = self.random_pos(4, 4) {
                self.solution.push(pos);
            }
        }
    }

    fn add_solution(&mut self) {
        if self.solution.len() > 9 {
            return;
        }
        if let Some(pos) = self.random_pos(4, 4) {
            self.solution.push(pos);
        }
    }

    fn random_pos(&mut self, max_y: usize, max_z: usize) -> Option<SimonSaysButton> {
        if self.solution.len() > 9 {
            return None;
        }

        let random_pos = SimonSaysButton::new(
            0,
            self.rng.random_range(0..max_y as i32),
            self.rng.random_range(0..max_z as i32),
        );

        if self.contains(random_pos) {
            self.random_pos(max_y, max_z) // Recursive call to try again
        } else {
            Some(random_pos)
        }
    }

    fn contains(&self, pos: SimonSaysButton) -> bool {
        self.solution.iter().any(|p| {
            p.x == pos.x && p.y == pos.y && p.z == pos.z
        })
    }

    pub fn reset(&mut self, complete: bool) {
        self.solution.clear();
        self.progress = 0;
        self.start_clicks = 0;
        self.is_skip = false;
        self.showing_solution = false;
        self.completed = complete;
    }

    pub fn tick(&mut self, _world: &mut World) {
        self.clicked_this_tick = false;
        
        if self.showing_solution && !self.solution.is_empty() {
            // TODO: Handle solution display timing
        }
    }
}

impl Default for SimonSays {
    fn default() -> Self {
        Self::new()
    }
}

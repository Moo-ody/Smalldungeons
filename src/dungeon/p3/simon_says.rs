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
    Completed,
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
    pub solution: Vec<SimonSaysButton>,
    start_time: Option<u64>,
    last_clicked: Option<u64>,
    pub showing_solution: bool,
    pub is_skip: bool,
    pub clicked_this_tick: bool,
    rng: rand::rngs::ThreadRng,
    
    // Timing system for solution display
    solution_display_start: Option<u64>,
    solution_display_step: usize,
    pub pending_actions: Vec<(u64, SolutionAction)>,
    
    // Split timing system
    clicking_phases: Vec<(u64, u64)>, // (first_click_time, last_click_time) for each clicking phase
    current_clicking_start: Option<u64>, // When the current clicking phase started
    puzzle_start_time: Option<u64>, // When the puzzle actually started (after first solution display)
}

#[derive(Debug, Clone)]
pub enum SolutionAction {
    ShowSeaLantern(BlockPos),
    HideSeaLantern(BlockPos),
    ReplaceButtons,
    StartPuzzle,
}

// Constants from Java code
pub const BOT_LEFT: SimonSaysButton = SimonSaysButton { x: 110, y: 120, z: 92 };
pub const START_BUTTON: SimonSaysButton = SimonSaysButton { x: 110, y: 121, z: 91 };

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
            
            // Timing system
            solution_display_start: None,
            solution_display_step: 0,
            pending_actions: Vec::new(),
            
            // Split timing system
            clicking_phases: Vec::new(),
            current_clicking_start: None,
            puzzle_start_time: None,
        }
    }

    pub fn should_run(&self) -> bool {
        // Only run if puzzle is not completed and we're in the Simon Says area
        // TODO: Check if we're in P3 phase
        !self.completed
    }
    
    pub fn is_in_simon_says_area(&self, pos: BlockPos) -> bool {
        // Check if the click is in the Simon Says button area
        // Simon Says area is around coordinates 110, 120-123, 91-95
        pos.x >= 110 && pos.x <= 113 &&
        pos.y >= 120 && pos.y <= 123 &&
        pos.z >= 91 && pos.z <= 95
    }

    pub fn handle_button_click(&mut self, pos: BlockPos, player_id: ClientId, current_tick: u64) -> Option<SimonSaysAction> {
        println!("Simon Says: Button clicked at {:?}", pos);
        
        if !self.should_run() {
            println!("Simon Says: Puzzle should not run (completed: {})", self.completed);
            return None;
        }
        
        if !self.is_in_simon_says_area(pos) {
            println!("Simon Says: Click not in Simon Says area");
            return None;
        }

        // Prevent multiple clicks in the same tick to avoid race conditions
        if self.clicked_this_tick {
            println!("Simon Says: Blocking click - already clicked this tick");
            return Some(SimonSaysAction::BlockClick);
        }

        println!("Simon Says: showing_solution: {}, solution_len: {}, progress: {}", 
                self.showing_solution, self.solution.len(), self.progress);

        if self.showing_solution {
            println!("Simon Says: Blocking click during solution display");
            return Some(SimonSaysAction::BlockClick); // Block clicks during solution display
        }

        if self.is_start_button(pos) {
            println!("Simon Says: Start button clicked");
            self.handle_start_button_click(current_tick);
            return Some(SimonSaysAction::ShowSolution);
        } else if !self.solution.is_empty() && self.is_solution_button(pos) {
            println!("Simon Says: Correct solution button clicked (progress {})", self.progress);
            let old_progress = self.progress;
            let old_solution_len = self.solution.len();
            self.handle_solution_button_click(player_id, current_tick);
            
            // Check if puzzle is completed
            if self.completed {
                return Some(SimonSaysAction::Completed);
            }
            
            // Check if we completed a sequence and need to show the next solution
            // We completed a sequence if:
            // 1. We were at the last button of the sequence (old_progress == old_solution_len - 1)
            // 2. Progress was reset to 0 (indicating we completed the sequence)
            // 3. A new solution was added (solution length increased)
            println!("Simon Says: Checking sequence completion - old_progress: {}, old_solution_len: {}, new_progress: {}, new_solution_len: {}", 
                    old_progress, old_solution_len, self.progress, self.solution.len());
            
            if old_progress == old_solution_len - 1 && self.progress == 0 && self.solution.len() > old_solution_len {
                // We just completed a sequence, show the new solution
                println!("Simon Says: Sequence completed, showing new solution");
                return Some(SimonSaysAction::ShowSolution);
            }
            
            return Some(SimonSaysAction::Continue);
        } else if !self.solution.is_empty() {
            println!("Simon Says: Wrong button clicked (expected progress {})", self.progress);
            self.fail(player_id);
            return Some(SimonSaysAction::Fail);
        }

        println!("Simon Says: No action taken for button click");
        None
    }

    fn is_start_button(&self, pos: BlockPos) -> bool {
        pos == START_BUTTON.to_block_pos()
    }

    fn is_solution_button(&self, pos: BlockPos) -> bool {
        if self.progress >= self.solution.len() {
            println!("Simon Says: Progress {} >= solution length {}", self.progress, self.solution.len());
            return false;
        }

        let expected_pos = self.solution[self.progress].to_block_pos();
        let actual_pos = BOT_LEFT.to_block_pos().add(expected_pos);
        
        println!("Simon Says: Checking solution button - clicked: {:?}, expected: {:?}, BOT_LEFT: {:?}", 
                pos, expected_pos, BOT_LEFT.to_block_pos());
        println!("Simon Says: Calculated actual_pos: {:?}, match: {}", actual_pos, pos == actual_pos);
        
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

        // Start a new clicking phase when the first button is clicked in a sequence
        if self.progress == 1 {
            self.start_clicking_phase(current_tick);
        }

        println!("Simon Says: Button click handled - progress: {}, solution_len: {}", self.progress, self.solution.len());

        // TODO: Play success sound
        // Utils.sendPacket(player, new S29PacketSoundEffect("note.pling", ...));

        if self.progress >= 5 {
            // Puzzle completed
            println!("Simon Says: Puzzle completed!");
            if let Some(start_time) = self.start_time {
                let total_time = (current_tick * 50 - start_time) as f64 / 1000.0;
                // TODO: Send completion message
            }
            
            // End the current clicking phase
            self.end_clicking_phase(current_tick);
            
            // Mark as completed but don't reset yet - we need the timing data for the completion message
            self.completed = true;
            // TODO: Mark terminal as completed
        } else if self.progress >= self.solution.len() {
            // Sequence completed, add next button
            println!("Simon Says: Sequence completed, adding next button. Old solution len: {}", self.solution.len());
            
            // End the current clicking phase
            self.end_clicking_phase(current_tick);
            
            self.progress = 0;
            self.add_solution();
            println!("Simon Says: New solution len: {}", self.solution.len());
            // The solution will be shown by the caller when it detects this condition
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
        
        // Schedule solution display actions
        self.pending_actions.clear();
        let current_tick = world.tick_count;
        
        for (i, button_pos) in self.solution.iter().enumerate() {
            let display_pos = BOT_LEFT.to_block_pos().add(button_pos.to_block_pos()).add(BlockPos::new(1, 0, 0));
            
            // Show sea lantern at position i * 8 ticks
            self.pending_actions.push((current_tick + (i * 8) as u64, SolutionAction::ShowSeaLantern(display_pos)));
            
            // Hide sea lantern at (i + 1) * 8 ticks
            self.pending_actions.push((current_tick + ((i + 1) * 8) as u64, SolutionAction::HideSeaLantern(display_pos)));
        }
        
        // Replace buttons after showing solution
        let replace_tick = current_tick + (8 * (self.solution.len() + 1)) as u64;
        self.pending_actions.push((replace_tick, SolutionAction::ReplaceButtons));
        
        self.solution_display_start = Some(current_tick);
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
                world.set_block_at(Blocks::StoneButton { direction: crate::server::block::block_parameter::ButtonDirection::from_meta(2), powered: false }, pos.x, pos.y, pos.z);
            }
        }
    }

    pub fn get_button_positions(&self) -> Vec<BlockPos> {
        let mut positions = Vec::new();
        for y in 0..4 {
            for z in 0..4 {
                let pos = BOT_LEFT.to_block_pos().add(BlockPos::new(0, y, z));
                positions.push(pos);
            }
        }
        positions
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
        
        // Clear timing system
        self.solution_display_start = None;
        self.solution_display_step = 0;
        self.pending_actions.clear();
        
        // Clear split timing system
        self.clicking_phases.clear();
        self.current_clicking_start = None;
        self.puzzle_start_time = None;
    }

    pub fn tick(&mut self) {
        self.clicked_this_tick = false;
        // The actual action processing will be handled by the world tick method
        // This method just resets the clicked_this_tick flag
    }
    
    pub fn start_puzzle(&mut self, current_tick: u64) {
        // Record when the puzzle actually starts (after first solution display)
        if self.puzzle_start_time.is_none() {
            self.puzzle_start_time = Some(current_tick);
            println!("Simon Says: Puzzle started at tick {}", current_tick);
        }
    }
    
    pub fn start_clicking_phase(&mut self, current_tick: u64) {
        // Start a new clicking phase when the first button is clicked in a sequence
        self.current_clicking_start = Some(current_tick);
        println!("Simon Says: Started clicking phase {} at tick {}", self.clicking_phases.len() + 1, current_tick);
    }
    
    pub fn end_clicking_phase(&mut self, current_tick: u64) {
        // End the current clicking phase when the sequence is completed
        if let Some(clicking_start) = self.current_clicking_start {
            self.clicking_phases.push((clicking_start, current_tick));
            println!("Simon Says: Ended clicking phase {} at tick {} (duration: {:.1}s)", 
                    self.clicking_phases.len(), current_tick, (current_tick - clicking_start) as f64 / 20.0);
        }
        self.current_clicking_start = None;
    }
    
    pub fn get_splits_string(&self) -> Option<String> {
        if self.clicking_phases.is_empty() {
            return None;
        }
        
        let mut splits = Vec::new();
        
        // Calculate splits (time for each clicking phase - only the actual clicking time)
        for &(clicking_start, clicking_end) in &self.clicking_phases {
            let clicking_duration = (clicking_end - clicking_start) as f64 / 20.0; // Convert ticks to seconds
            splits.push(clicking_duration);
        }
        
        // Calculate total time from puzzle start to completion (including all waiting)
        let total_time = if let (Some(puzzle_start), Some((_, last_clicking_end))) = (self.puzzle_start_time, self.clicking_phases.last()) {
            (last_clicking_end - puzzle_start) as f64 / 20.0
        } else {
            0.0
        };
        
        // Format splits as "0.5 | 1.3 | 1.5 | 1.8 | Total: 4.1s"
        let splits_str = splits.iter()
            .map(|&split| format!("{:.1}", split))
            .collect::<Vec<_>>()
            .join(" | ");
        
        Some(format!("{} | Total: {:.1}s", splits_str, total_time))
    }
}

impl Default for SimonSays {
    fn default() -> Self {
        Self::new()
    }
}

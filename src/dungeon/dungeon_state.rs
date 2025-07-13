
// just a test for now
#[derive(Debug)]
pub enum DungeonState {
    NotReady,
    Starting { tick_countdown: u16 },
    Started { current_ticks: u64 }, 
    Finished,
}

// probably should put all data when dungeon is started here
#[derive(Debug)]
pub enum DungeonState {
    NotReady,
    Starting { tick_countdown: u16 },
    Started {
        current_ticks: u64 
    }, 
    Finished,
}
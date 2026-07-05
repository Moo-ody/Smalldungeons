//! Tiny reusable per-ability cooldown counter, following the same hand-rolled
//! tick-decrement-per-`World::tick()` pattern already used by `AttackCooldown`/`AISuspended`
//! in `spawn_equipped.rs` - there is no repeating-timer primitive elsewhere to build on.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cooldown {
    ticks_remaining: u32,
}

impl Cooldown {
    pub const fn ready() -> Self {
        Self { ticks_remaining: 0 }
    }

    pub fn tick(&mut self) {
        self.ticks_remaining = self.ticks_remaining.saturating_sub(1);
    }

    pub fn is_ready(&self) -> bool {
        self.ticks_remaining == 0
    }

    pub fn trigger(&mut self, reset_to: u32) {
        self.ticks_remaining = reset_to;
    }
}

//! A reusable, world-level reimplementation of real vanilla 1.8.9 water physics
//! (`net.minecraft.block.BlockDynamicLiquid`/`BlockLiquid`), scoped to a caller-provided exact
//! set of valid world-space cells (`WaterSim::new`'s own `valid_cells` - a plain positive
//! membership set, not a bounding volume with holes carved out of it: a cell not in the set is
//! invalid for this sim, full stop, regardless of direction or code path). Any puzzle/room that
//! needs believable flowing water (currently just
//! Water Board, see `crate::dungeon::room::waterboard`) owns one [`WaterSim`] and drives it
//! from its own per-tick hook; the sim itself knows nothing about levers, gates, or any other
//! puzzle concept - it only ever reacts to (a) sources being added/removed and (b)
//! [`WaterSim::notify_neighbors`] being told a plain block changed nearby, exactly like real
//! vanilla's own `notifyNeighborsOfStateChange` waking up adjacent liquid blocks.
//!
//! Faithful to real 1.8 water in the ways that actually matter for how it *looks and behaves*:
//! - A cell is either a real source (in `sources`, rendered as `StillWater{level: 0}`, never
//!   recomputed - a source is a source until something else removes it, matching "only the
//!   originally placed water is a source") or a plain computed `FlowingWater{level}` cell whose
//!   4-bit vanilla metadata (`level`) packs both the 0-7 decay AND the falling flag (`0x8`) in
//!   the same byte, exactly like the real `LEVEL` blockstate (0-15) it's a 1:1 stand-in for.
//! - Falling (fed directly from above) always recomputes to decay 0 with the falling bit set;
//!   horizontal spread always increases decay by 1 from the weakest supplying neighbor, capped
//!   at [`MAX_DECAY`], and prefers whichever direction(s) have the real shortest path to a
//!   drop-off (a recursive, corner-aware search up to [`DROP_OFF_LOOKAHEAD`] blocks - not a
//!   straight-line-only approximation), falling back to spreading into every open direction only
//!   when no drop-off is found within range, same as real vanilla.
//! - Falling always pre-empts horizontal spread the same update (a cell that can fall never also
//!   spreads sideways that tick).
//! - Every recompute/spread is driven by a per-position scheduled-update queue with a fixed
//!   5-tick delay (real water's own `tickRate`), processed incrementally every world tick rather
//!   than resolving an entire flood in one pass - a cut source dries out and a newly opened path
//!   fills in over several waves, cascading outward one hop per delay, exactly like real vanilla.
//! - A block update is only ever sent for a cell whose water state actually changed.
//! - The one real vanilla "infinite water source" mechanic (a non-source water cell with >=2
//!   horizontal source neighbors AND a solid-or-source floor promotes itself to a new permanent
//!   source) is implemented exactly as that specific condition - never any other time, so two
//!   unrelated flows meeting head-on can't accidentally manufacture a new source.

use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::world::World;
use std::collections::{HashMap, HashSet};

/// Real vanilla water's own max decay: a source (or falling water) is decay 0, each horizontal
/// hop away from a supply adds 1, and a computed decay past this point means "no valid supply
/// found" - the cell has nothing to draw from and reverts to air. Also the real `level`
/// metadata's own low-3-bit range (0-7), used directly.
pub const MAX_DECAY: u8 = 7;

/// Raw metadata bit marking a `FlowingWater` cell as "falling" (fed directly from the block
/// above) - matches real vanilla's own `LEVEL` blockstate (0-15): values 8-15 are the falling
/// counterpart of decay values 0-7. This project only ever produces/consumes falling water at
/// decay 0 (real vanilla effectively always does too - a falling column has no reason to ever
/// carry a nonzero decay), so this sim always writes/reads the falling bit alone (`0x8`).
const FALLING_BIT: u8 = 0x8;

/// Real vanilla water's own fixed re-check delay (a quarter second, `BlockLiquid.tickRate`) -
/// every scheduled recompute/spread waits this many ticks before it actually runs, which is what
/// makes water visibly crawl forward one hop per wave instead of the whole flood resolving
/// instantly.
const TICK_DELAY: u64 = 5;

/// How far real vanilla water's own recursive search looks for a drop-off (a spot whose floor is
/// open) before giving up and spreading evenly instead - real vanilla's own constant.
const DROP_OFF_LOOKAHEAD: i32 = 4;

const HORIZONTAL: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// A reusable, room/volume-scoped vanilla water simulation - see the module doc comment.
#[derive(Debug)]
pub struct WaterSim {
    /// Permanent source cells, rendered as `StillWater{level: 0}` - never recomputed by
    /// [`Self::update_cell`], only ever added/removed by the owning puzzle (or by this sim's own
    /// real vanilla infinite-source promotion, see [`Self::maybe_promote_to_source`]).
    sources: HashSet<BlockPos>,
    /// Positions with a pending recompute/spread, and the tick they become due - real vanilla's
    /// own scheduled-block-update queue, deduplicated to the single soonest requested time per
    /// position (matching `World.isBlockTickPending`'s own dedup).
    scheduled: HashMap<BlockPos, u64>,
    /// The exact, exhaustive set of world-space cells this sim is ever allowed to place, move, or
    /// search through water in - not a bounding volume with exceptions carved out of it, a plain
    /// positive membership set: a cell not in here is invalid, full stop, regardless of how it's
    /// reached or what direction the attempt comes from. The owning puzzle builds this set once
    /// (typically from its own real captured geometry, transformed into world space) and hands it
    /// to [`Self::new`] - `WaterSim` itself never grows or shrinks it.
    valid: HashSet<BlockPos>,
    /// Real puzzle "output"/sink cells - see [`Self::add_terminal`]'s own doc comment.
    terminals: HashSet<BlockPos>,
}

impl WaterSim {
    /// `valid_cells` is the exact, exhaustive set of cells this sim may ever touch - see
    /// [`Self::valid`]'s own doc comment. Every internal operation (`schedule`, recompute, fall,
    /// horizontal spread, the drop-off search) checks membership in this same set before placing
    /// or queuing anything, via [`Self::in_bounds`], so a destination outside it can never receive
    /// simulated water no matter which direction or code path reaches for it.
    pub fn new(valid_cells: HashSet<BlockPos>) -> Self {
        Self { sources: HashSet::new(), scheduled: HashMap::new(), valid: valid_cells, terminals: HashSet::new() }
    }

    fn in_bounds(&self, pos: BlockPos) -> bool {
        self.valid.contains(&pos)
    }

    /// Marks `pos` as a real puzzle output/sink rather than an ordinary channel cell: it still
    /// fills and drains completely normally (falling water from directly above reaches it and
    /// shows up exactly like any other cell via the same [`Self::update_cell`] recompute, and
    /// retracts the same way if that supply later disappears - nothing extra needed for either),
    /// but it never promotes to a source and never spreads to ANY neighbor - not sideways, not
    /// down - and its own water is never counted as a horizontal supply source for a neighbor's
    /// recompute either. Without this, several real, independent detection points sitting along
    /// the same open row would silently become one shared trough the instant any one of them
    /// filled, purely because the space between them happens to be open air - this makes each one
    /// a true dead end fed only by whatever is directly above it. Not specific to any one puzzle
    /// concept (gates, levers, materials) - `WaterSim` still knows nothing about those; this is
    /// just a plain cell classification any caller can apply to its own real terminal points.
    pub fn add_terminal(&mut self, pos: BlockPos) {
        self.terminals.insert(pos);
    }

    /// Whether this sim currently owns any source or still has pending recomputes - lets a
    /// caller skip ticking entirely once everything's fully drained back to air.
    pub fn is_active(&self) -> bool {
        !self.sources.is_empty() || !self.scheduled.is_empty()
    }

    /// Places a new real, permanent source at `pos` (a `StillWater{level: 0}` block) and wakes
    /// it and its neighbors up for the next wave - the one and only way (besides the real
    /// vanilla infinite-source rule) a cell ever becomes a source in this sim.
    pub fn add_source(&mut self, world: &mut World, pos: BlockPos, now: u64) {
        self.sources.insert(pos);
        set_block_if_changed(world, pos, Blocks::StillWater { level: 0 });
        self.schedule(pos, now);
        self.schedule_neighbors(pos, now);
    }

    /// Removes a source, turning its cell back to plain air (matching a real piston physically
    /// displacing a source block) and waking its neighbors - real vanilla decay recomputation
    /// (driven entirely by [`Self::update_cell`] over the next several waves) does the rest for
    /// free: any cell that was only supplied by this source finds it gone, dries out, and wakes
    /// *its* own neighbors in turn, cascading the retraction outward one hop per
    /// [`TICK_DELAY`], exactly like real vanilla.
    pub fn remove_source(&mut self, world: &mut World, pos: BlockPos, now: u64) {
        if self.sources.remove(&pos) {
            set_block_if_changed(world, pos, Blocks::Air);
            self.schedule_neighbors(pos, now);
        }
    }

    /// Tells the sim a plain (non-fluid) block at `pos` just changed - real vanilla's own
    /// `notifyNeighborsOfStateChange`, which is what actually wakes up water sitting next to a
    /// gate/piston/door the instant it opens or closes, rather than waiting on that water's own
    /// unrelated schedule. The caller (Water Board) only ever needs to call this when ITS OWN
    /// gate geometry changes - it never has to touch decay/falling/spreading itself.
    pub fn notify_neighbors(&mut self, world: &World, pos: BlockPos, now: u64) {
        self.schedule_neighbors(pos, now);
        if is_water(world.get_block_at(pos.x, pos.y, pos.z)) {
            self.schedule(pos, now);
        }
    }

    fn schedule(&mut self, pos: BlockPos, now: u64) {
        if !self.in_bounds(pos) {
            return;
        }
        let due = now + TICK_DELAY;
        self.scheduled.entry(pos).and_modify(|t| *t = (*t).min(due)).or_insert(due);
    }

    fn schedule_neighbors(&mut self, pos: BlockPos, now: u64) {
        self.schedule(BlockPos::new(pos.x, pos.y + 1, pos.z), now);
        self.schedule(BlockPos::new(pos.x, pos.y - 1, pos.z), now);
        for (dx, dz) in HORIZONTAL {
            self.schedule(BlockPos::new(pos.x + dx, pos.y, pos.z + dz), now);
        }
    }

    /// Advances every currently-due scheduled position by one real vanilla update - called once
    /// per world tick by the owning puzzle; positions not yet due are left pending. Collects the
    /// due set up front (rather than draining `scheduled` while iterating) since a single cell's
    /// own update can itself re-schedule its neighbors for a LATER tick, which must not be
    /// visited again this same call.
    pub fn tick(&mut self, world: &mut World, now: u64) {
        let due: Vec<BlockPos> = self.scheduled.iter()
            .filter(|&(_, &t)| t <= now)
            .map(|(&pos, _)| pos)
            .collect();
        for pos in due {
            self.scheduled.remove(&pos);
            self.update_cell(world, pos, now);
        }
    }

    /// One real vanilla `updateTick` for a single water cell: recompute its own correct
    /// decay/falling state from its CURRENT real neighbors (sources are pinned, never
    /// recomputed), dry out to air with no valid supply, otherwise apply the real vanilla
    /// infinite-source check, then always try to spread from whatever the cell's final state is.
    fn update_cell(&mut self, world: &mut World, pos: BlockPos, now: u64) {
        if !self.in_bounds(pos) {
            return;
        }

        let current = world.get_block_at(pos.x, pos.y, pos.z);
        let is_source = self.sources.contains(&pos);
        if !is_water(current) {
            // Something else (a closing gate, a piston) already overwrote this cell - forget
            // it if it was a source (matches a real piston physically displacing one, see
            // `remove_source`) and leave nothing left here to recompute or spread from.
            self.sources.remove(&pos);
            return;
        }

        if !is_source {
            match self.compute_supply(world, pos) {
                Some((decay, falling)) => {
                    let raw = if falling { decay | FALLING_BIT } else { decay };
                    set_block_if_changed(world, pos, Blocks::FlowingWater { level: raw });
                    if raw != flowing_raw_level(current) {
                        self.schedule_neighbors(pos, now);
                    }
                }
                None => {
                    set_block_if_changed(world, pos, Blocks::Air);
                    self.schedule_neighbors(pos, now);
                    return;
                }
            }

            if self.terminals.contains(&pos) {
                // A real output: it just received (or kept) whatever falls into it from directly
                // above - see `add_terminal` - and stops right here, never promoting to a source
                // or spreading any further.
                return;
            }

            self.maybe_promote_to_source(world, pos, now);
        }

        self.try_spread(world, pos, now);
    }

    /// Real vanilla's own "infinite water source" rule, applied nowhere else: a non-source water
    /// cell with at least 2 horizontal neighbors that are real sources, sitting over a solid (or
    /// also-source) floor, permanently becomes a new source itself. This is the exact, narrow
    /// condition real vanilla uses - the only place this sim ever creates a new source outside
    /// of the owning puzzle's own explicit `add_source` call.
    fn maybe_promote_to_source(&mut self, world: &mut World, pos: BlockPos, now: u64) {
        if self.sources.contains(&pos) {
            return;
        }
        let source_neighbors = HORIZONTAL.iter()
            .filter(|&&(dx, dz)| self.is_source_block(world, BlockPos::new(pos.x + dx, pos.y, pos.z + dz)))
            .count();
        if source_neighbors < 2 {
            return;
        }
        let below = BlockPos::new(pos.x, pos.y - 1, pos.z);
        let below_block = world.get_block_at(below.x, below.y, below.z);
        if is_solid(below_block) || self.is_source_block(world, below) {
            self.sources.insert(pos);
            set_block_if_changed(world, pos, Blocks::StillWater { level: 0 });
            self.schedule_neighbors(pos, now);
        }
    }

    fn is_source_block(&self, world: &World, pos: BlockPos) -> bool {
        self.sources.contains(&pos) || matches!(world.get_block_at(pos.x, pos.y, pos.z), Blocks::StillWater { level: 0 })
    }

    /// Real vanilla spread: falling always pre-empts horizontal spread the same update and
    /// always lands at decay 0; otherwise, below [`MAX_DECAY`], spread horizontally at
    /// `decay + 1` into whichever open neighbor(s) have the real shortest path to a drop-off
    /// (see [`Self::flow_cost`]), or every open neighbor if none has one within range.
    fn try_spread(&mut self, world: &mut World, pos: BlockPos, now: u64) {
        if self.terminals.contains(&pos) {
            return;
        }
        let level = own_decay(world.get_block_at(pos.x, pos.y, pos.z));

        let below = BlockPos::new(pos.x, pos.y - 1, pos.z);
        if self.in_bounds(below) && self.can_flow_into(world, below, 0) {
            set_block_if_changed(world, below, Blocks::FlowingWater { level: FALLING_BIT });
            self.schedule(below, now);
            return;
        }

        if level >= MAX_DECAY {
            return;
        }
        let new_level = level + 1;

        let open: Vec<(i32, i32)> = HORIZONTAL.iter().copied()
            .filter(|&(dx, dz)| {
                let n = BlockPos::new(pos.x + dx, pos.y, pos.z + dz);
                self.in_bounds(n) && self.can_flow_into(world, n, new_level)
            })
            .collect();
        if open.is_empty() {
            return;
        }

        let costs: Vec<i32> = open.iter().map(|&(dx, dz)| self.flow_cost(world, pos, dx, dz)).collect();
        let best_cost = *costs.iter().min().unwrap();
        let chosen: Vec<(i32, i32)> = if best_cost < i32::MAX {
            open.iter().zip(&costs).filter(|&(_, &c)| c == best_cost).map(|(&d, _)| d).collect()
        } else {
            open
        };

        for (dx, dz) in chosen {
            let n = BlockPos::new(pos.x + dx, pos.y, pos.z + dz);
            set_block_if_changed(world, n, Blocks::FlowingWater { level: new_level });
            self.schedule(n, now);
        }
    }

    /// Real vanilla's own recursive, corner-aware search (up to [`DROP_OFF_LOOKAHEAD`] blocks)
    /// for the nearest drop-off reachable from `pos` by first stepping `(dx, dz)` - unlike a
    /// straight-line-only lookahead, this can find a hole around a bend, which is exactly what
    /// makes water in a turning channel correctly funnel toward the real exit instead of falling
    /// back to "no drop-off found -> spread every direction" at every corner.
    fn flow_cost(&self, world: &World, pos: BlockPos, dx: i32, dz: i32) -> i32 {
        self.flow_cost_search(world, BlockPos::new(pos.x + dx, pos.y, pos.z + dz), 1, (dx, dz))
    }

    fn flow_cost_search(&self, world: &World, pos: BlockPos, distance: i32, came_from: (i32, i32)) -> i32 {
        if !self.in_bounds(pos) {
            return i32::MAX;
        }
        // A "hole" is just an open floor (air, or already water) - a pure geometry check, not a
        // placement one, so this doesn't care whether the water below happens to already be
        // stronger than what would eventually flow into it.
        let below = BlockPos::new(pos.x, pos.y - 1, pos.z);
        if self.in_bounds(below) && is_passable(world.get_block_at(below.x, below.y, below.z)) {
            return distance;
        }
        // Likewise, the search itself passes freely through air OR existing water (real vanilla's
        // own `isBlocked` check is about solidity, not liquid level) - using the stricter
        // placement check here would make the search stop dead at the very first already-wet
        // cell it meets on a later wave, right back to "no drop-off found -> flood everywhere".
        if distance >= DROP_OFF_LOOKAHEAD || !is_passable(world.get_block_at(pos.x, pos.y, pos.z)) {
            return i32::MAX;
        }

        let mut best = i32::MAX;
        for (ndx, ndz) in HORIZONTAL {
            if (ndx, ndz) == (-came_from.0, -came_from.1) {
                continue;
            }
            let cost = self.flow_cost_search(world, BlockPos::new(pos.x + ndx, pos.y, pos.z + ndz), distance + 1, (ndx, ndz));
            best = best.min(cost);
        }
        best
    }

    /// Whether real water at the given new decay could actually occupy `pos` right now: real
    /// air, or existing tracked/real flowing water strictly weaker than the incoming decay
    /// (matching real vanilla - stronger water overwrites weaker, never the reverse). A real
    /// source is never open, matching a source's own permanence.
    fn can_flow_into(&self, world: &World, pos: BlockPos, new_decay: u8) -> bool {
        match world.get_block_at(pos.x, pos.y, pos.z) {
            Blocks::Air => true,
            Blocks::FlowingWater { level } => own_decay_raw(level) > new_decay,
            _ => false,
        }
    }

    /// Real vanilla's own two decay rules for one non-source water cell, given its CURRENT real
    /// neighbors: falling (fed directly from above) always wins and is always decay 0; otherwise
    /// the new decay is `1 +` the lowest decay among its 4 horizontal water neighbors, or `None`
    /// (no valid supply - the cell should revert to air) if that would exceed [`MAX_DECAY`] or
    /// there's no horizontal water neighbor at all. A terminal cell (see [`Self::add_terminal`])
    /// only ever takes the falling branch - it can't derive supply from a horizontal neighbor,
    /// and a terminal neighbor is skipped as a candidate supply for anyone else's horizontal
    /// check too, so a real output is fed only from directly above and never leaks its water
    /// sideways into whatever's next to it, terminal or not.
    fn compute_supply(&self, world: &World, pos: BlockPos) -> Option<(u8, bool)> {
        let above = BlockPos::new(pos.x, pos.y + 1, pos.z);
        if is_water(world.get_block_at(above.x, above.y, above.z)) {
            return Some((0, true));
        }

        if self.terminals.contains(&pos) {
            return None;
        }

        let mut best: Option<u8> = None;
        for (dx, dz) in HORIZONTAL {
            let n = BlockPos::new(pos.x + dx, pos.y, pos.z + dz);
            if self.terminals.contains(&n) {
                continue;
            }
            if let Some(decay) = neighbor_supply_decay(world.get_block_at(n.x, n.y, n.z)) {
                best = Some(best.map_or(decay, |b: u8| b.min(decay)));
            }
        }

        match best {
            Some(decay) if decay < MAX_DECAY => Some((decay + 1, false)),
            _ => None,
        }
    }
}

fn is_water(block: Blocks) -> bool {
    matches!(block, Blocks::StillWater { .. } | Blocks::FlowingWater { .. })
}

fn is_solid(block: Blocks) -> bool {
    !matches!(block, Blocks::Air) && !is_water(block)
}

/// Whether the real drop-off search (`flow_cost_search`) can treat `block` as open space - air
/// or ANY existing water, regardless of level. Deliberately looser than `can_flow_into` (which
/// governs actual placement): real vanilla's own search only cares about solidity, not about
/// whether the water already there happens to be stronger than what might eventually flow in.
fn is_passable(block: Blocks) -> bool {
    matches!(block, Blocks::Air) || is_water(block)
}

/// This cell's own decay for spread purposes (ignoring the falling bit - a falling cell is
/// always decay 0 already, see the module doc comment); sources are decay 0.
fn own_decay(block: Blocks) -> u8 {
    match block {
        Blocks::StillWater { .. } => 0,
        Blocks::FlowingWater { level } => own_decay_raw(level),
        _ => MAX_DECAY,
    }
}

fn own_decay_raw(level: u8) -> u8 {
    level & MAX_DECAY
}

fn flowing_raw_level(block: Blocks) -> u8 {
    match block {
        Blocks::FlowingWater { level } => level,
        _ => u8::MAX,
    }
}

/// The real decay a neighbor at `pos` would supply, for [`compute_supply`]'s own horizontal
/// scan - a falling neighbor supplies full pressure (decay 0) horizontally too, matching real
/// vanilla's well-known "every level of a waterfall can also spread sideways as if it were a
/// source" behavior.
fn neighbor_supply_decay(block: Blocks) -> Option<u8> {
    match block {
        Blocks::StillWater { level } => Some(own_decay_raw(level)),
        Blocks::FlowingWater { level } => Some(if level & FALLING_BIT != 0 { 0 } else { own_decay_raw(level) }),
        _ => None,
    }
}


fn set_block_if_changed(world: &mut World, pos: BlockPos, block: Blocks) {
    if world.get_block_at(pos.x, pos.y, pos.z) != block {
        world.set_block_at(block, pos.x, pos.y, pos.z);
    }
}


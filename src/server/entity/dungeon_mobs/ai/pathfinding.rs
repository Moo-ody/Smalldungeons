//! Room-scoped A* pathfinding over a 2.5D column/floor-height grid, following vanilla's
//! `WalkNodeProcessor` shape (open air cheap, no floor beneath = blocked, hazards blocked) but
//! bounded to the mob's current room and budget-capped, since nothing in this codebase throttles
//! per-mob AI cost otherwise (`World::tick` ticks every entity unconditionally every tick).
//!
//! Scope: only pathing within a single room is supported. If the mob and its goal are in
//! different rooms, `find_path` returns `None` and `next_steer_point` falls back to steering
//! straight at the goal - today's pre-pathfinding behavior, unchanged. Cross-room routing
//! through doorways is real follow-up work, not handled here.
//!
//! `find_path` never treats "physically closest to the goal" as good enough on its own - if the
//! exact goal column can't be reached (an impassable height difference, no connected route),
//! it returns a path to the *closest node the search actually confirmed reachable* instead of
//! either the unreachable goal or nothing at all. A mob standing on an elevated ledge with a
//! player below (or vice versa) should make the search head for a staircase and climb it, not
//! walk to the one block geometrically nearest the target and stop - that "closest in a
//! straight line" fallback (steering straight at the raw goal with zero pathfinding) is exactly
//! what happens when this module has nothing to offer, so minimizing how often that happens is
//! the actual point of the "best reachable node" fallback below.

use crate::server::block::block_collision::is_block_passable;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::dungeon_mobs::ai::cooldown::Cooldown;
use crate::server::entity::entity::EntityId;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Ticks between path recomputation for a given mob (0.5s @ 20 TPS) - pathfinding is
/// expensive enough (bounded but non-trivial block-lookup budget) that it must not run every
/// tick per mob, unlike direct steering. Mirrors the `Cooldown` pattern already used by
/// `idle_wander_cooldown`/`attack_cooldowns` in `state.rs`.
const RECOMPUTE_INTERVAL_TICKS: u32 = 10;
/// Force an early recompute if the goal has moved this far since the last search (target
/// walked away from where the path was aimed).
const RECOMPUTE_GOAL_DELTA: f64 = 3.0;
/// How close (horizontally) to a waypoint counts as "reached it" - advance to the next one.
const WAYPOINT_ARRIVAL_DISTANCE: f64 = 0.6;
/// If a mob hasn't moved (horizontally) more than this in a tick, it isn't making progress.
const STUCK_MOVEMENT_EPSILON: f64 = 0.02;
/// Ticks of no horizontal progress before forcing an early path recompute, bypassing the normal
/// `RECOMPUTE_INTERVAL_TICKS` cooldown - a mob pushed up against a local minimum (e.g. the
/// current path/waypoint genuinely can't be completed, or fallback direct-steering has it
/// wedged against a wall) shouldn't just keep failing silently until the next scheduled
/// recompute.
const STUCK_TICKS_THRESHOLD: u32 = 20;
/// Hard cap on A* node expansions per search - bounds worst-case per-tick cost regardless of
/// room size. Set to comfortably cover a full 2x2-segment room (~64x64 columns = 4096 possible
/// nodes) rather than a smaller value that trades completeness for speed: a smaller cap
/// (500, tried initially) reliably failed to find real routes that have to detour away from
/// the straight-line heuristic direction before making progress - e.g. a player standing on a
/// ledge next to a staircase on the *other* side of the room, where every node near the base
/// of the ledge scores better on Manhattan-XZ distance than the staircase does, so the search
/// exhausted its budget on that dead end before ever reaching the real route and fell back to
/// direct steering, which just walks a mob to the wall under the ledge and stops. Each node
/// expansion is a handful of O(1) block lookups (`ChunkGrid::get_block_at`, no allocation), so
/// even the full 4096-node worst case is cheap, especially throttled to once every
/// `RECOMPUTE_INTERVAL_TICKS` per mob rather than every tick.
const MAX_EXPANDED_NODES: usize = 4096;
/// Matches `physics::STEP_HEIGHT` (0.6, i.e. a 1-block step) - a neighbor floor exactly 1
/// block higher is still a free edge, same as the existing auto-step in `try_move_axis`.
const MAX_STEP_UP: i32 = 1;
/// How far down a neighbor's floor may drop and still be considered a normal walkable edge
/// (not a "cliff") - beyond this the node simply isn't connected. This is the actual fix for
/// "mob walks off a ledge chasing a player on the far side of a gap": the A* graph never
/// contains an edge onto/through the drop, so a route around it is preferred, or (if there's
/// no route) `find_path` fails and movement falls back to direct steering.
const MAX_SAFE_DROP: i32 = 3;

/// Cached path state for one dungeon mob - lives in `World.entity_mob_path` rather than on
/// `MobAiState`, since `MobAiState` is `Copy` by design (see `state.rs`) and `Vec<DVec3>`
/// can't be.
#[derive(Clone)]
pub struct MobPath {
    /// World-space waypoints (feet position, one per node, centered on the block), start to
    /// goal - the goal itself only if it was actually reachable, otherwise the closest node
    /// the search confirmed reachable (see `find_path`). Empty means "no path at all" (the mob
    /// hasn't made any real progress toward the goal yet) - callers fall back to steering
    /// straight at the goal in that case only.
    waypoints: Vec<DVec3>,
    current_index: usize,
    recompute_cooldown: Cooldown,
    last_goal: DVec3,
    /// The mob's own position as of the last tick - compared against the current position to
    /// detect "not making progress" for `stuck_ticks` below.
    last_position: DVec3,
    /// Consecutive ticks with no meaningful horizontal movement - see `STUCK_TICKS_THRESHOLD`.
    stuck_ticks: u32,
}

impl Default for MobPath {
    // Manual impl, not `#[derive(Default)]` - `DVec3` doesn't implement `Default`.
    fn default() -> Self {
        Self {
            waypoints: Vec::new(),
            current_index: 0,
            recompute_cooldown: Cooldown::ready(),
            last_goal: DVec3::ZERO,
            last_position: DVec3::ZERO,
            stuck_ticks: 0,
        }
    }
}

/// Per-tick entry point: advances/recomputes the mob's cached path as needed, and returns the
/// point `movement::steer_toward` should actually walk toward - the next waypoint toward
/// wherever `find_path` actually confirmed reachable (the goal itself, or the closest reachable
/// point short of it), or `goal` directly only when the mob is in a different room from it, or
/// hasn't made any real progress toward it at all yet - i.e. today's pre-pathfinding
/// direct-steering behavior, kept as a last resort rather than the common case.
///
/// The returned `bool` is `true` only when the returned point is an actual resolved waypoint
/// (not the `goal` fallback) - callers thread it through as `allow_jump`
/// (`movement::apply_movement_style`/`physics::move_horizontal`) so a mob only jumps because the
/// path it's following calls for it, not as a reflex during direct-line fallback steering.
///
/// Removes then reinserts the (non-`Copy`) `MobPath` around the `find_path` call, mirroring
/// how `run_mob_ai` snapshots/writes-back the (`Copy`) `MobAiState` - `find_path` only needs
/// `&World` (via the same `world.server_mut()` escape hatch `green_room.rs` already uses), but
/// this function takes `&mut World` to update the cache, and a live `&mut` borrow into
/// `world.entity_mob_path` can't coexist with holding `path` out of it at the same time.
pub fn next_steer_point(world: &mut World, entity_id: EntityId, mob_pos: DVec3, goal: DVec3, width: f64, height: f64) -> (DVec3, bool) {
    let mut path = world.entity_mob_path.remove(&entity_id).unwrap_or_default();
    path.recompute_cooldown.tick();

    if horizontal_distance(mob_pos, path.last_position) < STUCK_MOVEMENT_EPSILON {
        path.stuck_ticks += 1;
    } else {
        path.stuck_ticks = 0;
    }
    path.last_position = mob_pos;
    let stuck = path.stuck_ticks >= STUCK_TICKS_THRESHOLD;

    // Being stuck forces a recompute regardless of the normal cooldown/goal-delta gating below
    // - a mob wedged against a local minimum (its current waypoint genuinely unreachable, or
    // direct-steering fallback pinned against a wall) shouldn't just keep failing silently
    // until the next scheduled recompute.
    let needs_recompute = stuck
        || (path.recompute_cooldown.is_ready()
            && (path.current_index >= path.waypoints.len()
                || path.last_goal.distance_to(&goal) > RECOMPUTE_GOAL_DELTA));

    if needs_recompute {
        path.last_goal = goal;
        path.recompute_cooldown.trigger(RECOMPUTE_INTERVAL_TICKS);
        path.stuck_ticks = 0;
        path.waypoints = find_path(world, mob_pos, goal, width, height).unwrap_or_default();
        path.current_index = 0;
    }

    // Horizontal-only distance, deliberately not `DVec3::distance_to` (which includes Y) - a
    // waypoint's Y assumes a full-block step (`node_to_waypoint`'s `node.y + 1`), but a
    // stair/slab tread's real surface is only half that, and `physics::move_horizontal`'s
    // step-up doesn't land exactly on it either (it steps by a flat `STEP_HEIGHT`, not the
    // block's actual height). That mismatch is small but was permanent - `distance_to`'s
    // vertical component alone could sit at ~0.5, already exceeding
    // `WAYPOINT_ARRIVAL_DISTANCE`, so a mob approaching a stair/slab waypoint could get
    // horizontally right on top of it and *still* never count as having arrived, freezing
    // `current_index` on that one waypoint forever - which reads as "climbs one stair, then
    // stops" on a multi-step staircase. `steer_toward` itself already only steers on X/Z for
    // exactly this reason (Y is gravity/step's job, not steering's).
    while path.current_index < path.waypoints.len()
        && horizontal_distance(mob_pos, path.waypoints[path.current_index]) < WAYPOINT_ARRIVAL_DISTANCE {
        path.current_index += 1;
    }

    let on_real_waypoint = path.current_index < path.waypoints.len();
    let steer_point = path.waypoints.get(path.current_index).copied().unwrap_or(goal);
    world.entity_mob_path.insert(entity_id, path);
    (steer_point, on_real_waypoint)
}

/// Room-scoped A*. Node key is a `BlockPos` where `.y` is the floor block's Y (not the mob's
/// feet Y) - keying by column+floor rather than just column lets a genuinely multi-level room
/// (e.g. a bridge over a pit) be represented correctly, at negligible extra cost (`BlockPos`
/// already derives `Hash`/`Eq`).
fn find_path(world: &World, start: DVec3, goal: DVec3, width: f64, height: f64) -> Option<Vec<DVec3>> {
    let dungeon = &world.server_mut().dungeon;
    let start_room = dungeon.get_room_at(start.x.floor() as i32, start.z.floor() as i32)?;
    let goal_room = dungeon.get_room_at(goal.x.floor() as i32, goal.z.floor() as i32)?;
    if start_room != goal_room {
        return None;
    }
    let room = dungeon.rooms.get(start_room)?;
    let (min, max) = room.get_world_bounds();

    // Deliberately just 1, not `height.ceil()` (which would be 2 for a ~1.95-tall mob): real
    // dungeon staircases are routinely built with tight, sloped ceiling clearance that only
    // leaves 1 clear block directly above each individual step's own column, even though the
    // corridor as a whole is tall enough for a mob to actually walk up it (the same way vanilla
    // player collision doesn't demand a full extra clear block over every discrete stair
    // tread). Requiring 2 rejected legitimate stairs/slabs outright - too strict is worse here
    // than too lenient, since this is only a coarse per-column approximation of continuous 3D
    // collision to begin with (the real per-tick `physics::move_horizontal` collision check
    // still applies on top of whatever path this returns, so this can't let a mob clip through
    // a genuinely solid low ceiling).
    let clearance = 1;
    let _ = height;
    let _ = width; // mob width isn't modeled per-node (columns are single-block wide) - the
                   // existing per-tick collision layer (`physics::move_horizontal`) still
                   // enforces real hitbox-vs-wall collision on top of whatever path this returns.

    let start_floor = floor_near(world, min.y, max.y, start.x.floor() as i32, start.z.floor() as i32, start.y.floor() as i32, clearance)?;
    // Not `?` here, deliberately - if the goal's own column somehow doesn't resolve to a
    // standable floor (an edge case, but not one worth failing the whole search over), fall
    // back to the goal's raw (unvalidated) Y purely to steer the heuristic in the right
    // direction. `goal_node` is never required to be reachable itself - the search below always
    // settles for the closest node it actually confirmed reachable if it can't get there
    // exactly, so an approximate goal_node still points the search the right way.
    let goal_floor = floor_near(world, min.y, max.y, goal.x.floor() as i32, goal.z.floor() as i32, goal.y.floor() as i32, clearance)
        .unwrap_or(goal.y.floor() as i32);

    let start_node = BlockPos::new(start.x.floor() as i32, start_floor, start.z.floor() as i32);
    let goal_node = BlockPos::new(goal.x.floor() as i32, goal_floor, goal.z.floor() as i32);

    let mut open: BinaryHeap<Reverse<HeapEntry>> = BinaryHeap::new();
    let mut best_g: HashMap<BlockPos, f32> = HashMap::new();
    let mut parent: HashMap<BlockPos, BlockPos> = HashMap::new();

    best_g.insert(start_node, 0.0);
    open.push(Reverse(HeapEntry { cost: OrderedCost(heuristic(start_node, goal_node)), node: start_node }));

    let mut expanded = 0usize;
    // Tracks the confirmed-reachable node (one actually inserted into `best_g`, i.e. connected
    // to `start_node` via a real chain of valid edges) with the smallest remaining
    // heuristic-to-goal seen so far - the fallback target if the exact goal is never reached.
    // Starts at `start_node` itself (trivially "reachable", distance zero progress) so "no
    // progress was made at all" is distinguishable from "made it partway".
    let mut closest_reachable = start_node;
    let mut closest_reachable_h = heuristic(start_node, goal_node);

    // Nodes already popped and expanded once - without this, a node relaxed more than once
    // (any node with more than one incoming edge, which is most of them on a 4-directional grid)
    // gets a fresh heap entry pushed each time it's relaxed, and every one of those duplicates
    // still counts against `MAX_EXPANDED_NODES` when it's eventually popped, even though only
    // the first (cheapest) pop ever does anything new. In an open room this burned a large chunk
    // of the budget re-expanding nodes near the start/under-player area over and over, so a
    // route that has to detour any real distance (e.g. across the room to a staircase) could run
    // the search dry before ever reaching it - "closer to the stairs it works, mid-distance it
    // doesn't" is exactly that: less waste to burn through before the real route is in range.
    let mut closed: HashSet<BlockPos> = HashSet::new();

    while let Some(Reverse(HeapEntry { node: current, .. })) = open.pop() {
        if !closed.insert(current) {
            continue; // stale duplicate entry for a node already finalized - not real work
        }

        // Full `BlockPos` equality (X, Y, *and* Z) - not just the XZ column. `floor_in_window`
        // lets each step drop up to `MAX_SAFE_DROP` (3) blocks, repeatable hop after hop, so the
        // search can and does reach the goal's column at a much lower floor - e.g. the ground
        // node directly beneath an elevated player - long before it ever reaches the actual
        // standable floor next to them. An XZ-only check here declared that lower node "the
        // goal" and stopped, which is precisely the "mob walks to the spot right under the
        // player and just stands there" bug: it never got a chance to explore on toward the
        // staircase because it thought it had already arrived. `best_g`/`parent` are already
        // keyed by full `BlockPos` (this is intentionally a 2.5D graph - see the node-key comment
        // above `find_path`), so requiring the Y to match `goal_node`'s resolved standable floor
        // too is just being consistent with how every other node in this graph is identified.
        if current == goal_node {
            return Some(reconstruct(&parent, current));
        }
        expanded += 1;
        if expanded > MAX_EXPANDED_NODES {
            break;
        }

        let g = best_g[&current];
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let nx = current.x + dx;
            let nz = current.z + dz;
            if nx < min.x || nx > max.x || nz < min.z || nz > max.z {
                continue;
            }
            let Some(neighbor_floor) = floor_in_window(world, current.y, nx, nz, clearance) else { continue };
            let neighbor = BlockPos::new(nx, neighbor_floor, nz);
            if closed.contains(&neighbor) {
                continue; // already finalized with its best cost - re-pushing it is pure waste
            }
            let tentative_g = g + 1.0;
            if tentative_g < *best_g.get(&neighbor).unwrap_or(&f32::INFINITY) {
                best_g.insert(neighbor, tentative_g);
                parent.insert(neighbor, current);
                let neighbor_h = heuristic(neighbor, goal_node);
                if neighbor_h < closest_reachable_h {
                    closest_reachable_h = neighbor_h;
                    closest_reachable = neighbor;
                }
                open.push(Reverse(HeapEntry { cost: OrderedCost(tentative_g + neighbor_h), node: neighbor }));
            }
        }
    }

    // Never reached the exact goal column (search budget exhausted, or the open set emptied
    // because the graph is genuinely disconnected from here - e.g. a real height difference
    // with no connecting route). Return a path to the closest node the search actually
    // confirmed reachable instead of nothing - this is what actually fixes "walks to the block
    // geometrically nearest the target and just stands there": `None` here previously made
    // `next_steer_point` fall back to blind straight-line steering at the raw (possibly
    // unreachable) goal, which has no concept of walls or height differences at all. A path to
    // `closest_reachable` routes through real, confirmed-walkable terrain instead (e.g. toward
    // the base of a staircase, if that's as far as this search got before running out of
    // budget) - and since `next_steer_point` recomputes periodically (or immediately if the mob
    // gets stuck), later searches from further along that route can keep making progress rather
    // than a single search needing to solve the whole thing in one shot.
    if closest_reachable == start_node {
        return None; // never made any real progress at all - genuinely nothing useful to offer
    }
    Some(reconstruct(&parent, closest_reachable))
}

/// 3D Manhattan distance (X + Z + Y). Deliberately includes `Y`, not just the XZ grid: this
/// heuristic is reused both to order the A* search *and*, more importantly, to pick
/// `closest_reachable` when the exact goal column can't be reached (see `find_path`). An
/// XZ-only heuristic would let a node with zero horizontal distance but a huge vertical gap -
/// e.g. standing directly under an elevated player, unable to reach them - always "win" as
/// closest, reproducing the exact "closest position geometrically instead of a reachable one"
/// bug this is meant to fix, just one layer down. Folding `Y` in means a node that's made real
/// vertical progress up a staircase scores better than one sitting uselessly underneath the
/// target. This makes the heuristic inadmissible for strict shortest-path optimality (a single
/// step can change `y` by more than 1 during a stair climb, so it can overestimate), but
/// completeness/correctness of the *endpoint* chosen matters far more here than path optimality,
/// and the search is bounded by `MAX_EXPANDED_NODES` regardless.
fn heuristic(a: BlockPos, b: BlockPos) -> f32 {
    ((a.x - b.x).abs() + (a.z - b.z).abs() + (a.y - b.y).abs()) as f32
}

/// XZ-only distance between two world positions, ignoring Y entirely - see the comment at
/// `next_steer_point`'s waypoint-arrival check for why this matters.
fn horizontal_distance(a: DVec3, b: DVec3) -> f64 {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    (dx * dx + dz * dz).sqrt()
}

/// Finds a standable floor at `(x, z)` within `[MAX_SAFE_DROP below, MAX_STEP_UP above]` of
/// `from_y` - i.e. "is this neighbor column reachable as a single step from the current node's
/// floor", not an unbounded scan. Returns `None` (blocked) if nothing qualifies - this is what
/// actually prevents a mob from stepping onto/through a deep drop or a column with no floor at
/// all (an open pit).
fn floor_in_window(world: &World, from_y: i32, x: i32, z: i32, clearance: i32) -> Option<i32> {
    for y in (from_y - MAX_SAFE_DROP..=from_y + MAX_STEP_UP).rev() {
        if is_standable(world, x, y, z, clearance) {
            return Some(y);
        }
    }
    None
}

/// Same idea as `floor_in_window` but used only for the start/goal columns, where there's no
/// "current node" yet to window against - searches outward from the entity's actual feet Y
/// across the room's full Y range.
fn floor_near(world: &World, min_y: i32, max_y: i32, x: i32, z: i32, near_y: i32, clearance: i32) -> Option<i32> {
    for offset in 0..=(max_y - min_y).max(0) {
        for y in [near_y - offset, near_y + offset] {
            if y >= min_y && y <= max_y && is_standable(world, x, y, z, clearance) {
                return Some(y);
            }
        }
    }
    None
}

/// `y` is solid ground, non-hazardous, with `clearance` blocks of passable, non-lava headroom
/// above it.
fn is_standable(world: &World, x: i32, y: i32, z: i32, clearance: i32) -> bool {
    let floor_block = world.get_block_at(x, y, z);
    if is_block_passable(floor_block) || is_lava(floor_block) {
        return false; // no floor here (air/liquid) - this IS the ledge-fall fix
    }
    for dy in 1..=clearance {
        let above = world.get_block_at(x, y + dy, z);
        if !is_block_passable(above) || is_lava(above) {
            return false;
        }
    }
    true
}

/// Lava specifically (not water) - the one hazard that's passable to collision
/// (`block_collision::is_liquid` doesn't distinguish it from water) but must never be a path
/// node: this codebase's mobs have no fire-immunity data, and `physics.rs`'s buoyancy would
/// just have a mob drift/float in it, which looks worse than routing around.
fn is_lava(block: Blocks) -> bool {
    matches!(block, Blocks::FlowingLava { .. } | Blocks::Lava { .. })
}

fn reconstruct(parent: &HashMap<BlockPos, BlockPos>, mut current: BlockPos) -> Vec<DVec3> {
    let mut path = vec![node_to_waypoint(current)];
    while let Some(&p) = parent.get(&current) {
        path.push(node_to_waypoint(p));
        current = p;
    }
    path.reverse();
    path
}

fn node_to_waypoint(node: BlockPos) -> DVec3 {
    DVec3::new(node.x as f64 + 0.5, (node.y + 1) as f64, node.z as f64 + 0.5)
}

/// `f32` wrapper with a total-order `Ord`/`PartialOrd` so it can sit inside `BinaryHeap`'s
/// `Reverse` tuple key - plain `f32` isn't `Ord` (NaN has no defined order), but A* costs here
/// are always finite non-negative sums, so `total_cmp` is safe and avoids a bespoke min-heap
/// entry type.
#[derive(PartialEq)]
struct OrderedCost(f32);
impl Eq for OrderedCost {}
impl PartialOrd for OrderedCost {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrderedCost {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// Open-set entry for the `BinaryHeap` - orders purely by `cost`, ignoring `node` entirely
/// (`BlockPos` has no `Ord` impl, and doesn't need one just to break cost ties).
struct HeapEntry {
    cost: OrderedCost,
    node: BlockPos,
}
impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}
impl Eq for HeapEntry {}
impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cost.cmp(&other.cost)
    }
}

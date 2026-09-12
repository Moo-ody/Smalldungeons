use crate::net::protocol::play::clientbound::EntityVelocity;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::{ClientId, Player};
use crate::server::block::block_collision::get_block_aabb;
use crate::server::block::blocks::Blocks;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::aabb::AABB;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::var_int::VarInt;
use anyhow::Result;

/// Pearl entity implementation, ported from RustClear's simpler gravity/drag/AABB-overlap
/// physics (confirmed to feel better in practice than the previous raytrace+swept-AABB version).
pub struct PearlEntityImpl {
    thrower_client_id: ClientId,
    spawn_velocity: DVec3,
}

impl PearlEntityImpl {
    pub fn new(thrower_client_id: ClientId, velocity: DVec3) -> Self {
        Self {
            thrower_client_id,
            spawn_velocity: velocity,
        }
    }
}

impl EntityImpl for PearlEntityImpl {
    fn spawn(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        // Inform clients of initial motion so the projectile animates
        for _player in entity.world_mut().players.values() {
            let _ = packet_buffer.write_packet(&EntityVelocity {
                entity_id: entity.id,
                velocity_x: self.spawn_velocity.x,
                velocity_y: self.spawn_velocity.y,
                velocity_z: self.spawn_velocity.z,
            });
        }
        entity.velocity = self.spawn_velocity;
    }

    fn tick(&mut self, entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
        const GRAVITY: f64 = 0.03;
        const DRAG: f64 = 0.99;
        // Largest single substep this tick's movement gets divided into - well under both a
        // slab's half-block thickness and the pearl's own 0.25-wide hitbox, so a fast-moving
        // pearl (enough velocity that a whole tick's movement is several blocks) can never
        // tunnel past a thin obstacle - e.g. clipping through a slab from underneath - between
        // two consecutive collision checks the way a single start/end-of-tick check could.

        const MAX_SUBSTEP: f64 = 0.2;

        entity.velocity.y -= GRAVITY;
        entity.velocity *= DRAG;

        let travel = entity.velocity;
        let distance = (travel.x * travel.x + travel.y * travel.y + travel.z * travel.z).sqrt();
        let substeps = ((distance / MAX_SUBSTEP).ceil() as u32).max(1);
        let step = DVec3::new(travel.x / substeps as f64, travel.y / substeps as f64, travel.z / substeps as f64);

        let world = entity.world_mut();
        let mut current = entity.position;
        // `Blocks` of whatever was hit, plus the exact substep position the pearl was at when
        // the collision was registered (as opposed to `current`, which stays at the last *safe*
        // pre-collision substep) - iron bars need that raw impact position, see
        // `compute_landing_position`'s doc comment.
        let mut hit: Option<(Blocks, DVec3)> = None;

        for _ in 0..substeps {
            let candidate = current + step;
            let aabb = AABB::from_width_height(0.25, 0.25).offset(candidate);
            if let Some(hit_block) = check_block_collisions(world, &aabb) {
                hit = Some((hit_block, candidate));
                break;
            }
            current = candidate;
        }
        entity.position = current;

        if let Some((hit_block, impact_pos)) = hit {
            // Room-name lookup (not `.as i32` - see the falling-floor fix elsewhere in this
            // codebase for why truncation instead of `.floor()` silently breaks on the negative
            // coordinates real dungeon rooms use) to opt Cobblestone Walls in "New Trap" back out
            // of the clip-through landing - see `compute_landing_position`'s doc comment.
            let dungeon = &world.server_mut().dungeon;
            let in_new_trap = dungeon
                .get_room_at(impact_pos.x.floor() as i32, impact_pos.z.floor() as i32)
                .is_some_and(|room_index| dungeon.rooms[room_index].room_data.name == "New Trap");

            if let Some(player) = world.players.get_mut(&self.thrower_client_id) {
                let land_pos = compute_landing_position(hit_block, current, impact_pos, in_new_trap);
                // keep yaw/pitch, set absolute xyz - `server_teleport` handles both the
                // immediate authoritative position update (so chunks for the landing area get
                // sent right away) and rejecting stale pre-teleport position reports.
                player.server_teleport(land_pos, 0.0, 0.0, 24);
            }
            world.despawn_entity(entity.id);
            return;
        }

        const MAX_TICKS: u32 = 200;
        if entity.ticks_existed > MAX_TICKS {
            world.despawn_entity(entity.id);
        }
    }
}

/// Every block type pearls fly straight through as if it were air, never landing on or being
/// stopped by one - per explicit request: every real pressure plate variant, skulls, and
/// ladders. Scoped to this file's own local `check_block_collisions` (pearl-only) rather than
/// the shared `block_collision::get_block_aabb` every other system (mob physics/pathfinding,
/// other projectiles) also uses, so this doesn't change how anything else in the game treats them.
fn pearl_ignores(block: Blocks) -> bool {
    matches!(
        block,
        Blocks::StonePressurePlate { .. }
            | Blocks::WoodenPressurePlate { .. }
            | Blocks::GoldPressurePlate { .. }
            | Blocks::IronPressurePlate { .. }
            | Blocks::Skull { .. }
            | Blocks::Ladder { .. }
    )
}

/// Half-thickness of an iron-bar pane's post/arms, in blocks either side of the cell's own
/// center line - vanilla's real 2px (2/16) pane thickness, so a post/arm spans
/// `[0.5 - BAR_HALF_THICKNESS, 0.5 + BAR_HALF_THICKNESS]` on its thin axis.
const BAR_HALF_THICKNESS: f64 = 0.0625;

/// Whether an iron-bar cell at `(x, y, z)` connects to its neighbor `(x + dx, y, z + dz)` -
/// vanilla panes connect to another pane of the same kind, or to a solid full-cube neighbor
/// (checked via `get_block_aabb` spanning exactly that neighbor's own cell - a partial shape
/// like a slab or stairs does not count, matching vanilla).
fn iron_bar_connects(world: &crate::server::world::World, x: i32, y: i32, z: i32, dx: i32, dz: i32) -> bool {
    let (nx, nz) = (x + dx, z + dz);
    let neighbor = world.get_block_at(nx, y, nz);
    if matches!(neighbor, Blocks::IronBars) {
        return true;
    }
    match get_block_aabb(neighbor, nx, y, nz) {
        Some(a) => {
            a.min == DVec3::new(nx as f64, y as f64, nz as f64)
                && a.max == DVec3::new(nx as f64 + 1.0, y as f64 + 1.0, nz as f64 + 1.0)
        }
        None => false,
    }
}

/// The real, connection-aware collision shape of one iron-bar cell at `(x, y, z)`, as separate
/// world-space boxes: a thin center post (always present) plus one thin arm per side that
/// actually connects (see `iron_bar_connects`). Deliberately returned as a list rather than one
/// bounding box spanning all of them - unioning them would fill in whatever corners aren't
/// actually connected, which is exactly the "empty space inside the cell" a pearl needs to be
/// able to fly through. Every box is full cell height (vanilla bars are never partial-height).
fn iron_bar_components(world: &crate::server::world::World, x: i32, y: i32, z: i32) -> Vec<AABB> {
    let base = DVec3::new(x as f64, y as f64, z as f64);
    let lo = 0.5 - BAR_HALF_THICKNESS;
    let hi = 0.5 + BAR_HALF_THICKNESS;

    let mut boxes = vec![AABB::new(base + DVec3::new(lo, 0.0, lo), base + DVec3::new(hi, 1.0, hi))];

    if iron_bar_connects(world, x, y, z, 0, -1) {
        // north (-z): post out to the cell's near face
        boxes.push(AABB::new(base + DVec3::new(lo, 0.0, 0.0), base + DVec3::new(hi, 1.0, lo)));
    }
    if iron_bar_connects(world, x, y, z, 0, 1) {
        // south (+z)
        boxes.push(AABB::new(base + DVec3::new(lo, 0.0, hi), base + DVec3::new(hi, 1.0, 1.0)));
    }
    if iron_bar_connects(world, x, y, z, -1, 0) {
        // west (-x)
        boxes.push(AABB::new(base + DVec3::new(0.0, 0.0, lo), base + DVec3::new(lo, 1.0, hi)));
    }
    if iron_bar_connects(world, x, y, z, 1, 0) {
        // east (+x)
        boxes.push(AABB::new(base + DVec3::new(hi, 0.0, lo), base + DVec3::new(1.0, 1.0, hi)));
    }

    boxes
}

/// Any block whose AABB (accounting for partial shapes like slabs/liquids via `get_block_aabb`,
/// or iron bars' own connection-aware post/arm shape via `iron_bar_components`) overlaps the
/// given box - except whatever `pearl_ignores` always treats as air. Returns the block that was
/// hit (the first intersecting one found, scan order x/y/z) rather than just yes/no, so the
/// caller can special-case specific block types (iron bars, cobblestone walls) instead of always
/// applying the generic "land on top of the block" treatment.
fn check_block_collisions(world: &mut crate::server::world::World, aabb: &AABB) -> Option<Blocks> {
    let min_bx = aabb.min.x.floor() as i32;
    let max_bx = aabb.max.x.ceil() as i32;
    let min_by = aabb.min.y.floor() as i32;
    let max_by = aabb.max.y.ceil() as i32;
    let min_bz = aabb.min.z.floor() as i32;
    let max_bz = aabb.max.z.ceil() as i32;

    for bx in min_bx..=max_bx {
        for by in min_by..=max_by {
            for bz in min_bz..=max_bz {
                let block = world.get_block_at(bx, by, bz);
                if pearl_ignores(block) {
                    continue;
                }

                if matches!(block, Blocks::IronBars) {
                    // Real pane geometry - tested as separate components (never merged into one
                    // box) so a pearl through an unconnected corner never registers a hit.
                    let hit = iron_bar_components(world, bx, by, bz)
                        .iter()
                        .any(|component| aabb.intersects(component));
                    if hit {
                        return Some(block);
                    }
                    continue;
                }

                if let Some(block_aabb) = get_block_aabb(block, bx, by, bz) {
                    if aabb.intersects(&block_aabb) {
                        return Some(block);
                    }
                }
            }
        }
    }

    None
}

/// Computes where the thrower ends up after a pearl collision, given which block was hit.
///
/// Every block except Iron Bars (always) and Cobblestone Walls (everywhere *except* the "New
/// Trap" room - see below) gets the existing, unchanged treatment: land centered on top of the
/// block cell at the last *safe* (pre-collision) substep position (`safe_pos`) - vanilla-ish
/// "safely correct the player above/beside whatever they hit".
///
/// Iron Bars and (outside "New Trap") Cobblestone Walls are the deliberate exceptions, per
/// explicit request: both should let the pearl carry the player into/through them rather than
/// depositing them on top like a solid block. X/Z are still centered on the hit block's own cell
/// (`impact_pos`'s cell, not `safe_pos`'s - the pearl's actual impact, so it's centered on the
/// block it hit rather than whatever cell it was passing through the tick before), same
/// `floor + 0.5` as the normal case. Y is the one axis that stays raw - `impact_pos.y`
/// unmodified, no `+1.0` top-of-block offset - since that's what places the player's feet inside
/// the block's own vertical space instead of safely on top of it, letting them clip/fall through.
///
/// `in_new_trap` (whether the impact happened inside the "New Trap" room specifically) opts
/// Cobblestone Walls back out of that treatment there, per explicit request - Iron Bars is
/// unaffected by this flag and keeps the clip-through treatment everywhere, "New Trap" included.
fn compute_landing_position(hit_block: Blocks, safe_pos: DVec3, impact_pos: DVec3, in_new_trap: bool) -> DVec3 {
    let clips_through = match hit_block {
        Blocks::IronBars => true,
        Blocks::CobblestoneWalls { .. } => !in_new_trap,
        _ => false,
    };

    if clips_through {
        DVec3::new(
            impact_pos.x.floor() + 0.5,
            impact_pos.y,
            impact_pos.z.floor() + 0.5,
        )
    } else {
        DVec3::new(
            safe_pos.x.floor() + 0.5,
            safe_pos.y.floor() + 1.0,
            safe_pos.z.floor() + 0.5,
        )
    }
}

pub fn on_right_click(player: &mut Player) -> Result<()> {
    let eye_height = 1.62; // player eye height in blocks
    let eye_pos = DVec3::new(
        player.position.x,
        player.position.y + eye_height,
        player.position.z,
    );

    // Convert yaw/pitch (degrees) to a forward direction vector
    let yaw_rad = (player.yaw as f64).to_radians();
    let pitch_rad = (player.pitch as f64).to_radians();
    let dir = DVec3::new(
        -pitch_rad.cos() * yaw_rad.sin(),
        -pitch_rad.sin(),
        pitch_rad.cos() * yaw_rad.cos(),
    );
    let dir = dir.normalize();

    let velocity = DVec3::new(dir.x * 1.5, dir.y * 1.5, dir.z * 1.5); // Vanilla-ish speed
    let spawn_pos = DVec3::new(
        eye_pos.x + dir.x * 0.2,
        eye_pos.y + dir.y * 0.2,
        eye_pos.z + dir.z * 0.2,
    ); // slight offset in front of player

    player.world_mut().spawn_entity(
        spawn_pos,
        EntityMetadata::new(EntityVariant::EnderPearl),
        PearlEntityImpl::new(player.client_id, velocity),
    )?;

    Ok(())
}

/// Convenience wrapper for dungeon item handlers and other call sites.
/// This matches the `throw_pearl(player)` signature used elsewhere.
pub fn throw_pearl(player: &mut Player) -> Result<()> {
    on_right_click(player)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::block::block_parameter::StairDirection;
    use crate::server::block::metadata::u3;
    use crate::server::utils::direction::Direction;
    use crate::server::world::World;

    fn pearl_aabb_at(pos: DVec3) -> AABB {
        AABB::from_width_height(0.25, 0.25).offset(pos)
    }

    // Every `World::new()` eagerly allocates a fixed 256x256 chunk grid - and since
    // `ChunkSection` (`[u16; 4096]`) leaves the compiler no spare niche to shrink `Option<T>`
    // for the `None` case, that's full-size per slot even for chunks that never get touched:
    // ~8.6GB per `World`. cargo test runs `#[test]` fns in parallel by default, so one `World`
    // per test here previously meant that many simultaneous 8.6GB allocations - way past what
    // any machine has, and the actual cause of an intermittent allocation-failure crash seen
    // while developing this file. Not something to fix in `World`/`ChunkGrid` itself (out of
    // scope, and used as-is by the real single-instance server) - so instead, each test below
    // shares one `World` across several distinct, well-separated block coordinates rather than
    // constructing a fresh one per case.

    // --- iron bars: connection-aware pane shape (post, each connected arm, a straight run's
    // empty sides, an L-shaped empty corner, "inside the 1x1 cell" alone isn't a collision) and
    // the precise-impact landing (raw Y, centered X/Z) ---

    #[test]
    fn iron_bars_pane_shape_and_landing() {
        let mut world = World::new();

        // Case A (cell x=5): isolated bar - post only, no arms. Covers the center-post hit +
        // its landing math, and "merely entering the 1x1 cell isn't a collision".
        world.set_block_at(Blocks::IronBars, 5, 70, 5);

        // Case B (cell x=15): 4-way connected - post + all of north/south/west/east arms.
        world.set_block_at(Blocks::IronBars, 15, 70, 5);
        world.set_block_at(Blocks::IronBars, 15, 70, 4); // north
        world.set_block_at(Blocks::IronBars, 15, 70, 6); // south
        world.set_block_at(Blocks::IronBars, 14, 70, 5); // west
        world.set_block_at(Blocks::IronBars, 16, 70, 5); // east

        // Case C (cell x=25): straight N-S run - no east/west arms.
        world.set_block_at(Blocks::IronBars, 25, 70, 5);
        world.set_block_at(Blocks::IronBars, 25, 70, 4); // north
        world.set_block_at(Blocks::IronBars, 25, 70, 6); // south

        // Case D (cell x=35): L shape - only north/east connected, leaving the southwest
        // quadrant of the cell empty.
        world.set_block_at(Blocks::IronBars, 35, 70, 5);
        world.set_block_at(Blocks::IronBars, 35, 70, 4); // north
        world.set_block_at(Blocks::IronBars, 36, 70, 5); // east

        // --- A: hitting the center post ---
        // The substep just before the bars (safe) and the substep that actually clips into the
        // center post (impact) - the post spans x/z [5.4375, 5.5625], so this box (which spans
        // [5.45, 5.70] on x/z) genuinely overlaps it, not just the bar's outer 1x1 cell.
        let safe_pos = DVec3::new(5.05, 70.1, 4.92);
        let impact_pos = DVec3::new(5.45, 70.34, 5.45);
        let hit = check_block_collisions(&mut world, &pearl_aabb_at(impact_pos));
        assert_eq!(hit, Some(Blocks::IronBars), "post hit");

        let land_pos = compute_landing_position(hit.unwrap(), safe_pos, impact_pos, false);
        // X/Z centered on the impacted cell (dead center, x .5 / z .5) - not the raw impact
        // coordinate, and not the safe-substep's cell either.
        assert_eq!(land_pos.x, 5.5);
        assert_eq!(land_pos.z, 5.5);
        // Y is the one axis left raw - no top-of-block "+1.0" - so the player's feet end up
        // inside the bars' own vertical space instead of standing on top of them.
        assert_eq!(land_pos.y, impact_pos.y);
        assert_ne!(land_pos.y, safe_pos.y.floor() + 1.0);

        // --- A: "inside the 1x1 cell" alone is not a collision (isolated bar, no arms) ---
        let corner_of_cell = DVec3::new(5.05, 70.3, 5.05);
        assert_eq!(
            check_block_collisions(&mut world, &pearl_aabb_at(corner_of_cell)),
            None,
            "corner of the cell, nowhere near the post"
        );

        // --- B: each connected arm registers a collision ---
        // Each position sits entirely within one arm's own exclusive footprint - outside the
        // post's [x.4375, x.5625] range on the arm's long axis - so a hit here proves the arm
        // geometry itself is solid, not just the post.
        let north = DVec3::new(15.47, 70.3, 5.05); // z short of the post's z-min
        let south = DVec3::new(15.47, 70.3, 5.60); // z past the post's z-max
        let west = DVec3::new(15.05, 70.3, 5.47); // x short of the post's x-min
        let east = DVec3::new(15.60, 70.3, 5.47); // x past the post's x-max
        for (name, pos) in [("north", north), ("south", south), ("west", west), ("east", east)] {
            assert_eq!(
                check_block_collisions(&mut world, &pearl_aabb_at(pos)),
                Some(Blocks::IronBars),
                "{name} arm should register a collision"
            );
        }

        // --- C: passing beside a straight bar (no east/west arm to hit) ---
        let beside = DVec3::new(25.05, 70.3, 5.47);
        assert_eq!(
            check_block_collisions(&mut world, &pearl_aabb_at(beside)),
            None,
            "beside a straight N-S run, no west arm present"
        );

        // --- D: passing through an L-shaped empty corner ---
        let empty_corner = DVec3::new(35.05, 70.3, 5.60); // southwest: low x, high z
        assert_eq!(
            check_block_collisions(&mut world, &pearl_aabb_at(empty_corner)),
            None,
            "southwest corner, empty since only north/east are connected"
        );
    }

    #[test]
    fn iron_bars_landing_overlaps_the_bar_cell() {
        let bar_cell = (5, 70, 5);
        let impact_pos = DVec3::new(5.12, 70.34, 5.07);
        let land_pos = compute_landing_position(Blocks::IronBars, DVec3::new(5.0, 70.0, 5.0), impact_pos, false);

        assert_eq!(land_pos.x.floor() as i32, bar_cell.0);
        assert_eq!(land_pos.y.floor() as i32, bar_cell.1);
        assert_eq!(land_pos.z.floor() as i32, bar_cell.2);
    }

    // --- cobblestone walls get the exact same landing treatment as iron bars everywhere except
    // the "New Trap" room, where they revert to the original centered top-of-block landing;
    // every other block (including a normal full block) always keeps that original landing ---

    #[test]
    fn cobblestone_walls_and_normal_block_landing() {
        let mut world = World::new();
        world.set_block_at(Blocks::CobblestoneWalls { variant: 0 }, 5, 70, 5);
        world.set_block_at(Blocks::Stone { variant: 0 }, 15, 70, 5);

        let safe_pos = DVec3::new(5.05, 70.1, 4.92);
        let impact_pos = DVec3::new(5.12, 70.34, 5.07);
        let hit = check_block_collisions(&mut world, &pearl_aabb_at(impact_pos));
        assert_eq!(hit, Some(Blocks::CobblestoneWalls { variant: 0 }));
        let land_pos = compute_landing_position(hit.unwrap(), safe_pos, impact_pos, false);
        assert_eq!(land_pos.x, 5.5);
        assert_eq!(land_pos.z, 5.5);
        assert_eq!(land_pos.y, impact_pos.y);
        assert_ne!(land_pos.y, safe_pos.y.floor() + 1.0);

        let safe_pos = DVec3::new(15.3, 70.4, 5.6);
        let impact_pos = DVec3::new(15.35, 70.45, 5.65);
        let hit = check_block_collisions(&mut world, &pearl_aabb_at(impact_pos));
        assert_eq!(hit, Some(Blocks::Stone { variant: 0 }));
        let land_pos = compute_landing_position(hit.unwrap(), safe_pos, impact_pos, false);
        // Unchanged formula: floor(safe_pos) + 0.5 on x/z, +1.0 on y - and specifically NOT the
        // raw impact position (which is what iron bars/cobblestone walls alone should use).
        assert_eq!(land_pos, DVec3::new(15.5, 71.0, 5.5));
        assert_ne!(land_pos, impact_pos);
    }

    #[test]
    fn cobblestone_walls_revert_to_old_landing_in_new_trap_but_iron_bars_do_not() {
        let safe_pos = DVec3::new(5.05, 70.1, 4.92);
        let impact_pos = DVec3::new(5.12, 70.34, 5.07);

        // Cobblestone Walls in "New Trap" (in_new_trap = true): back to the original centered
        // top-of-block landing, same formula a normal block gets - per explicit request, this
        // room specifically keeps the pre-clip-through behavior.
        let wall_in_new_trap = compute_landing_position(
            Blocks::CobblestoneWalls { variant: 0 },
            safe_pos,
            impact_pos,
            true,
        );
        assert_eq!(wall_in_new_trap, DVec3::new(5.5, 71.0, 4.5));
        assert_ne!(wall_in_new_trap.y, impact_pos.y);

        // Cobblestone Walls outside "New Trap" (in_new_trap = false): unaffected, still the
        // clip-through landing.
        let wall_elsewhere = compute_landing_position(
            Blocks::CobblestoneWalls { variant: 0 },
            safe_pos,
            impact_pos,
            false,
        );
        assert_eq!(wall_elsewhere.y, impact_pos.y);

        // Iron Bars are NOT part of this exception - `in_new_trap` must have no effect on them,
        // clip-through everywhere, "New Trap" included.
        let bars_in_new_trap = compute_landing_position(Blocks::IronBars, safe_pos, impact_pos, true);
        let bars_elsewhere = compute_landing_position(Blocks::IronBars, safe_pos, impact_pos, false);
        assert_eq!(bars_in_new_trap, bars_elsewhere);
        assert_eq!(bars_in_new_trap.y, impact_pos.y);
    }

    // --- ignored blocks (pressure plates, skulls) and partial-shape blocks (slabs, stairs)
    // keep their existing, unchanged collision behavior ---

    #[test]
    fn ignored_and_partial_shape_blocks_unchanged() {
        let mut world = World::new();
        world.set_block_at(Blocks::StonePressurePlate { powered: false }, 5, 70, 5);
        world.set_block_at(Blocks::Skull { direction: Direction::Up, no_drop: false }, 6, 70, 5);
        // Bottom-half slab: solid only in the lower half of the cell (y in [70.0, 70.5)).
        world.set_block_at(Blocks::StoneSlab { variant: u3(0), top_half: false }, 15, 70, 5);
        // Bottom-half stairs simplify (per `get_stair_aabb`) to the same lower-half hitbox,
        // regardless of facing direction.
        world.set_block_at(Blocks::OakStairs { direction: StairDirection::North, top_half: false }, 25, 70, 5);

        assert_eq!(check_block_collisions(&mut world, &pearl_aabb_at(DVec3::new(5.1, 70.1, 5.1))), None);
        assert_eq!(check_block_collisions(&mut world, &pearl_aabb_at(DVec3::new(6.1, 70.1, 5.1))), None);

        // A pearl passing through a slab's/stairs' empty upper half must NOT collide...
        assert_eq!(check_block_collisions(&mut world, &pearl_aabb_at(DVec3::new(15.1, 70.6, 5.1))), None);
        assert_eq!(check_block_collisions(&mut world, &pearl_aabb_at(DVec3::new(25.1, 70.6, 5.1))), None);

        // ...but one actually inside the solid lower half must.
        assert_eq!(
            check_block_collisions(&mut world, &pearl_aabb_at(DVec3::new(15.1, 70.1, 5.1))),
            Some(Blocks::StoneSlab { variant: u3(0), top_half: false })
        );
        assert_eq!(
            check_block_collisions(&mut world, &pearl_aabb_at(DVec3::new(25.1, 70.1, 5.1))),
            Some(Blocks::OakStairs { direction: StairDirection::North, top_half: false })
        );
    }
}

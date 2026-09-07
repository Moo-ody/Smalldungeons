use crate::dungeon::door::{Door, DoorType};
use crate::dungeon::room::room::Room;
use crate::dungeon::room::room_data::{RoomData, RoomShape, RoomType::*};
use crate::server::block::block_parameter::Axis;
use std::cmp::{max, min};

const RED: u8 = 4 * 4 + 2;
const GREEN: u8 = 7 * 4 + 2;
const ORANGE: u8 = 15 * 4 + 2;
const PURPLE: u8 = 16 * 4 + 2;
const PINK: u8 = 20 * 4 + 2;
/// "Yellow" is this codebase's name for what the real dungeon API/Skytils call a Champion room
/// (`room_data.rs::RoomType::from_str` maps the scraped JSON's literal `"yellow"` type string to
/// this variant) - vanilla map color id 74. This used to be 122 (`30*4+2`, the same numeric
/// family as `ENTRANCE`/`GREEN`'s base color but a different shade), which isn't a value
/// Catlas's `RoomType.fromMapColor` recognizes at all (only 74 maps to `CHAMPION` there) - any
/// unrecognized side-color makes `scanTile` return `Unknown` for the whole tile, so the room
/// didn't exist to Catlas at all instead of just rendering with the wrong color.
const YELLOW: u8 = 18 * 4 + 2;
const BROWN: u8 = 63;

/// Undiscovered-room/unopened-door placeholder color (vanilla map color id 85 - also doubles as
/// the "NORMAL" room/door type id, since an unrevealed tile has no known type yet). Skytils'
/// Catlas (`DungeonMapColorParser.scanTile`) specifically checks for the literal value 85 to
/// register a tile as `RoomState.UNOPENED` - any other value (this used to be 47, a shade of the
/// same gray but numerically meaningless to Catlas) falls through to its `else` branch and gets
/// misread as `DISCOVERED`.
const GRAY: u8 = 21 * 4 + 1;
/// Cleared-but-secrets-missing checkmark color (vanilla map color id 34), distinct from
/// `GREEN`/30 (fully done). Skytils checks for exactly 34 to register `RoomState.CLEARED`
/// (`DungeonMapColorParser.scanTile`); this used to reuse a `WHITE`/58 constant that isn't a
/// value Catlas recognizes at all, so this intermediate state was never visible.
const CLEARED: u8 = 8 * 4 + 2;
/// Failed-puzzle color (vanilla map color id 18) - real Hypixel's own dungeon map, confirmed
/// directly from Skytils' `DungeonMapColorParser.kt` (`scanTile`): `centerColor == 18` maps to
/// `RoomState.FAILED`, but only for `RoomType.PUZZLE` (the same 18 means something unrelated -
/// `DISCOVERED` - for a Blood room, so this is deliberately puzzle-only, not a generic "18 always
/// means failed" reading). Distinct from `GREEN`/30 and `CLEARED`/34, which both mean solved.
const FAILED: u8 = 18;
/// Unopened-Wither-door/room placeholder (vanilla map color id 119) - same `UNOPENED` meaning to
/// Catlas as `GRAY`/85, but also specifically what `DoorType.fromMapColor` reads as `WITHER`, so
/// it doubles as a "this locked door ahead is a wither door" hint. Reused below for the "?"
/// glyph on the placeholder room too, which only needs to look black/dark to a human - the exact
/// shade doesn't matter there. This used to be plain black (116, shade 0 of the same base color)
/// which isn't one of Catlas's recognized codes.
const BLACK: u8 = 29 * 4 + 3;

const QUESTION_MARK_POSITIONS: [(usize, usize); 11] = [
    (0, 1), (1, 0), (2, 0), (3, 0), (4, 1), (4, 2), (3, 3), (2, 4), (2, 5), (2, 7), (2, 8),
];

const CHECKMARK_POSITIONS: [(usize, usize); 30] = [
    (7, 0), (8, 0), (6, 1), (7, 1), (8, 1), (5, 2), (6, 2), (7, 2), (4, 3), (5, 3),
    (6, 3), (0, 4), (1, 4), (3, 4), (4, 4), (5, 4), (0, 5), (1, 5), (2, 5), (3, 5),
    (4, 5), (0, 6), (1, 6), (2, 6), (3, 6), (1, 7), (2, 7), (3, 7), (1, 8), (2, 8),
];

/// Failed-puzzle X glyph, drawn in place of `CHECKMARK_POSITIONS` (same 9x9 footprint/origin, so
/// it lands on the same room-center pixel Skytils' `scanTile` samples for `FAILED`/color-18
/// detection). Unlike the checkmark shape above - reverse-engineered from an actual captured real
/// map - no real captured X glyph exists to copy pixel-for-pixel, so this is a plain 2px-thick
/// diagonal cross built to the same size/weight instead of a guessed-at "authentic" shape. The
/// color (`FAILED`/18) is the part that's actually verified real, not this exact pixel art.
const X_MARK_POSITIONS: [(usize, usize); 33] = [
    (0, 0), (1, 0), (7, 0), (8, 0),
    (0, 1), (1, 1), (7, 1), (8, 1),
    (1, 2), (2, 2), (6, 2), (7, 2),
    (2, 3), (3, 3), (5, 3), (6, 3),
    (3, 4), (4, 4), (5, 4),
    (2, 5), (3, 5), (5, 5), (6, 5),
    (1, 6), (2, 6), (6, 6), (7, 6),
    (0, 7), (1, 7), (7, 7), (8, 7),
    (0, 8), (8, 8),
];

pub struct DirtyMapRegion {
    pub min_x: usize,
    pub min_y: usize,
    pub max_x: usize,
    pub max_y: usize,
}

pub struct DungeonMap {
    pub map_data: [u8; 128 * 128],
    offset_x: usize,
    offset_y: usize,
    dirty_region: Option<DirtyMapRegion>
}

// room is 16x16 px
// gap is 4 px
// door is 5x4 px
impl DungeonMap {

    pub fn new(offset_x: usize, offset_y: usize) -> Self {
        Self {
            map_data: [0; 128 * 128],
            offset_x,
            offset_y,
            dirty_region: None,
        }
    }

    /// Converts a world position to this map's icon coordinate encoding (the player-position
    /// arrow - see `MapIcon` in `net::protocol::play::clientbound`). Vanilla doubles the 0..128
    /// pixel-space coordinate and re-centers it into a signed byte, which is exactly what
    /// Skytils' `MapUtils.kt` (`Vec4b.mapX`/`mapZ`) decodes back with `(b + 128) shr 1`.
    /// `dungeon_origin` is the world coordinate that maps to this dungeon's own pixel (0, 0) -
    /// taken as a parameter (callers pass `DUNGEON_ORIGIN`) rather than imported here, to avoid
    /// a `dungeon.rs` <-> `map.rs` dependency cycle.
    pub fn world_to_icon(&self, world_x: f64, world_z: f64, dungeon_origin: (i32, i32)) -> (i8, i8) {
        // 20 map pixels per 32-block room segment - the same ratio `draw_room` places rooms at
        // (`segment.x * 20`, for a segment that's 32 world blocks wide).
        const PX_PER_BLOCK: f64 = 20.0 / 32.0;
        let pixel_x = (world_x - dungeon_origin.0 as f64) * PX_PER_BLOCK + self.offset_x as f64;
        let pixel_z = (world_z - dungeon_origin.1 as f64) * PX_PER_BLOCK + self.offset_y as f64;
        let icon_x = (pixel_x * 2.0 - 128.0).round().clamp(i8::MIN as f64, i8::MAX as f64) as i8;
        let icon_z = (pixel_z * 2.0 - 128.0).round().clamp(i8::MIN as f64, i8::MAX as f64) as i8;
        (icon_x, icon_z)
    }

    pub fn get_updated_area(&mut self) -> Option<(DirtyMapRegion, Vec<u8>)> {
        if let Some(region) = self.dirty_region.take() {
            let width = region.max_x - region.min_x;
            let height = region.max_y - region.min_y;
            let mut pixels = Vec::with_capacity(width * height);

            for row in region.min_y..region.max_y {
                let start = row * 128 + region.min_x;
                let end = row * 128 + region.max_x;

                pixels.extend_from_slice(&self.map_data[start..end]);
            }
            return Some((region, pixels))
        }
        None
    }

    fn set_px(&mut self, x: usize, y: usize, color: u8) {
        let x = x + self.offset_x;
        let y = y + self.offset_y;
        debug_assert!(x < 128);
        debug_assert!(y < 128);

        let region = self.dirty_region.get_or_insert(DirtyMapRegion {
            min_y: 128,
            min_x: 128,
            max_x: 0,
            max_y: 0,
        });
        region.min_x = min(region.min_x, x);
        region.min_y = min(region.min_y, y);
        region.max_x = max(region.max_x, x + 1);
        region.max_y = max(region.max_y, x + 1);

        self.map_data[y * 128 + x] = color
    }

    fn fill_px(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: u8,
    ) {
        let x = x + self.offset_x;
        let y = y + self.offset_y;

        debug_assert!(x + width < 128);
        debug_assert!(y + height < 128);

        let region = self.dirty_region.get_or_insert(DirtyMapRegion {
            min_y: 128,
            min_x: 128,
            max_x: 0,
            max_y: 0,
        });
        region.min_x = min(region.min_x, x);
        region.min_y = min(region.min_y, y);
        region.max_x = max(region.max_x, x + width);
        region.max_y = max(region.max_y, y + height);

        for x in x..x+width {
            for y in y..y+height {
                self.map_data[y * 128 + x] = color
            }
        }
    }
    
    pub fn draw_room(&mut self, rooms: &[Room], doors: &[Door], room_index: usize) {
        // Validate room index before accessing
        if room_index >= rooms.len() {
            eprintln!("Warning: Room index {} out of bounds for rooms vector of length {}", room_index, rooms.len());
            return;
        }
        let room = &rooms[room_index];
        let color = get_room_color(&room.room_data);
        
        for segment in room.segments.iter() {
            let x = segment.x * 20;
            let y = segment.z * 20;
            
            self.fill_px(x, y, 16, 16, color);
            if room.segments.iter().find(|seg| seg.x == segment.x + 1 && seg.z == segment.z).is_some() {
                self.fill_px(x + 16, y, 4, 16, color);
            }
            if room.segments.iter().find(|seg| seg.x == segment.x && seg.z == segment.z + 1).is_some() {
                self.fill_px(x, y + 16, 16, 4, color);
            }
            
            for (index, neighbour) in segment.neighbours.iter().enumerate() {
                if neighbour.is_none() {
                    continue;
                }
                
                let (neighbour_room, door) = {
                    let neighbour = neighbour.as_ref().unwrap();
                    
                    // Validate indices before accessing
                    if neighbour.room_index >= rooms.len() {
                        eprintln!("Warning: Neighbour room index {} out of bounds for rooms vector of length {}", neighbour.room_index, rooms.len());
                        continue;
                    }
                    if neighbour.door_index >= doors.len() {
                        eprintln!("Warning: Door index {} out of bounds for doors vector of length {}", neighbour.door_index, doors.len());
                        continue;
                    }
                    
                    (&rooms[neighbour.room_index], &doors[neighbour.door_index])
                };
                
                let mut x = segment.x * 20 + 6;
                let mut y = segment.z * 20 + 6;
                
                match index {
                    0 => y -= 10,
                    1 => x += 10,
                    2 => y += 10,
                    3 => x -= 10,
                    _ => unreachable!()
                }
                
                let (width, height) = match door.direction {
                    Axis::X => (4, 5),
                    Axis::Z => (5, 4),
                    _ => unreachable!()
                };
                
                if neighbour_room.entered {
                    // Neighbour's own 16x16 box (plus its checkmark, if any) was already drawn
                    // by its own entry-triggered `draw_room` call - only the connecting door gap
                    // needs touching here.
                    let color = get_door_color(room, neighbour_room);
                    self.fill_px(x, y, width, height, color);
                } else if door.opened {
                    // Door has been opened (Wither/Blood key used) but the room beyond hasn't
                    // been walked into yet - the dungeon layout is already known server-side
                    // (`room_data` is set at generation time, not on entry), so reveal its real
                    // shape/type color now instead of the gray "?" placeholder below. This is
                    // what lets the map show an opened split before it's been explored, same as
                    // real Hypixel - only the neighbour's own `entered`-gated redraw (above) adds
                    // the clear/secrets checkmark once it's actually explored.
                    let color = get_door_color(room, neighbour_room);
                    self.fill_px(x, y, width, height, color);

                    let mut x = segment.x * 20;
                    let mut y = segment.z * 20;

                    match index {
                        0 => y -= 20,
                        1 => x += 20,
                        2 => y += 20,
                        3 => x -= 20,
                        _ => unreachable!()
                    }

                    let neighbour_color = get_room_color(&neighbour_room.room_data);
                    self.fill_px(x, y, 16, 16, neighbour_color);
                } else {
                    let color = match door.door_type {
                        DoorType::WITHER => BLACK,
                        DoorType::BLOOD => RED,
                        _ => GRAY,
                    };
                    self.fill_px(x, y, width, height, color);

                    let mut x = segment.x * 20;
                    let mut y = segment.z * 20;

                    match index {
                        0 => y -= 20,
                        1 => x += 20,
                        2 => y += 20,
                        3 => x -= 20,
                        _ => unreachable!()
                    }

                    self.fill_px(x ,y, 16, 16, GRAY);

                    for (qx, qy) in QUESTION_MARK_POSITIONS {
                        self.set_px(x + qx + 5, y + qy + 5, BLACK);
                    }
                }
            }
        }
        
        // fill in hole
        if room.room_data.shape == RoomShape::TwoByTwo {
            let x = room.segments[0].x * 20 + 16;
            let y = room.segments[0].z * 20 + 16;
            self.fill_px(x, y, 4, 4, color)
        }
        
        {
            // Skip checkmark for entrance room (green room) and fairy room (pink room)
            // They stay their color with no checkmark
            //
            // Otherwise, no checkmark at all (room just stays its base color, e.g. brown) until
            // every starred mob spawned into it has died - a room with no starred mobs starts
            // at 0 remaining, so it counts as cleared immediately. Once cleared, green if all
            // secrets are found (or the room has none), white if secrets are still missing.
            //
            // `mobs_spawned` matters here: a room queued behind the mob-spawn cycle also reads
            // `starred_mobs_remaining == 0` before its mobs actually exist (indistinguishable
            // from "genuinely has none") - without this check the checkmark would flash on the
            // instant a room is entered, before any of its starred mobs even spawned in.
            //
            // Trap rooms are a special case of that same problem: they have no starred mobs by
            // design (a `starred_mobs_remaining` of 0 there is permanent, not "not spawned yet"),
            // so the mob-based check alone would show them cleared the instant they're entered.
            // Real Hypixel ties a Trap room's clear state to grabbing one specific chest instead
            // (`Room::trap_completion_chest_pos`, set by `block_interact_action.rs`'s `Chest`
            // handler) - not any of its other secrets, not mob count.
            //
            // Puzzle rooms have the exact same problem (no starred mobs either) - tied instead to
            // `Room::puzzle_completed`, set once a puzzle is actually resolved (solved *or*
            // failed - either way it's done, matching real Hypixel: a failed puzzle still marks
            // the room complete, it just costs score - see `three_weirdos::interact_chest`).
            let is_cleared = if room.room_data.room_type == Trap {
                room.trap_completed
            } else if room.room_data.room_type == Puzzle {
                room.puzzle_completed
            } else {
                room.mobs_spawned && room.starred_mobs_remaining == 0
            };
            if room.room_data.room_type != Entrance && room.room_data.room_type != Fairy && is_cleared {
                let x = room.segments[0].x * 20 + 4;
                let y = room.segments[0].z * 20 + 4;

                // A failed puzzle gets the real red-X glyph/color instead of the checkmark -
                // every other "cleared" room (including a puzzle that was *solved*) keeps the
                // checkmark. `puzzle_failed` is only ever set on `Puzzle`-type rooms (see its
                // own doc comment), so no extra room-type check is needed here.
                if room.puzzle_failed {
                    for (cx, cy) in X_MARK_POSITIONS {
                        self.set_px(x + cx, y + cy, FAILED)
                    }
                } else {
                    let checkmark_color = if room.room_data.secrets == 0 || room.found_secrets >= room.room_data.secrets {
                        GREEN
                    } else {
                        CLEARED
                    };

                    for (cx, cy) in CHECKMARK_POSITIONS {
                        self.set_px(x + cx, y + cy, checkmark_color)
                    }
                }
            }
        }
    }
}

fn get_room_color(room_data: &RoomData) -> u8 {
    match room_data.room_type {
        Normal | Rare => BROWN,
        Puzzle => PURPLE,
        Trap => ORANGE,
        Fairy => PINK,
        Entrance => GREEN,
        Blood => RED,
        Yellow =>  YELLOW,
        Boss => RED, // Boss rooms get red color
    }
}

fn get_door_color(room: &Room, neighbour: &Room) -> u8 {
    match room.room_data.room_type {
        Puzzle | Trap | Blood | Yellow | Fairy | Boss => {
            return get_room_color(&room.room_data)
        }
        _ => {}
    };
    match neighbour.room_data.room_type {
        Puzzle | Trap | Blood | Yellow | Boss => {
            return get_room_color(&neighbour.room_data)
        }
        _ => {}
    };
    BROWN
}
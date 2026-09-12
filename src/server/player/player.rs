use crate::net::internal_packets::NetworkThreadMessage;
use crate::net::packets::packet::IdentifiedPacket;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::packets::packet_serialize::PacketSerializable;
use crate::net::protocol::play::clientbound::{Chat, OpenWindow, SetSlot, WindowItems};
use crate::server::entity::entity::EntityId;
use crate::server::player::container_ui::UI;
use crate::server::player::inventory::{Inventory, ItemSlot};
use crate::server::player::terminal::Terminal;
use crate::server::player::scoreboard::Scoreboard;
use crate::server::player::dungeon_stats::DungeonPlayerStats;
use crate::server::server::Server;
use crate::server::utils::aabb::AABB;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use std::collections::HashMap;
use std::ops::Index;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
use crate::server::items::item_stack::ItemStack;

/// type alias to represent a client's user id.
///
/// alias for a u32
pub type ClientId = u32;

// add uuid
#[derive(Debug, Clone)]
pub struct GameProfileProperty {
    pub value: String,
    pub signature: Option<String>
}

#[derive(Debug, Clone)]
pub struct GameProfile {
    pub uuid: Uuid,
    pub username: String,
    pub properties: HashMap<String, GameProfileProperty>
}

#[derive(Debug)]
pub struct Player {
    pub server: *mut Server,
    pub packet_buffer: PacketBuffer,
    pub network_tx: UnboundedSender<NetworkThreadMessage>,
    
    pub profile: GameProfile,
    pub client_id: ClientId,
    pub entity_id: EntityId,
    
    pub position: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    
    pub last_position: DVec3,   
    pub last_yaw: f32,
    pub last_pitch: f32,

    pub ticks_existed: u32,
    pub last_keep_alive: i32,
    pub ping: i32,

    pub is_sneaking: bool,

    pub inventory: Inventory,
    pub held_slot: u8,
    
    pub window_id: i8,
    pub current_ui: UI,
    // pub current_ui: UI,
    pub current_terminal: Option<Terminal>,

    pub sidebar: Scoreboard,
    
    // Bonzo Staff cooldown tracking
    pub bonzo_last_shot_tick: u64,
    
    // Jerry-Chine Gun cooldown tracking
    pub jerry_last_shot_tick: u64,

    /// Terminator's "Shot Cooldown: 0.5s" (10 ticks) - the real item's other stats (damage,
    /// crit, headshot bonus) have nothing to plug into since there's no player-vs-mob damage
    /// system in this codebase yet; the cooldown is the one number that actually gates behavior.
    pub terminator_last_shot_tick: u64,

    /// Hyperion right-click cooldown (1/6 second - doesn't divide evenly into this codebase's
    /// 20-tick-per-second loop the way Bonzo/Jerry's whole-tick cooldowns do, so this is
    /// tracked as a real timestamp instead of a tick count for actual precision).
    pub hyperion_last_used: Option<std::time::Instant>,

    /// World tick of the last `UseEntity` packet that fired the held item's right-click ability.
    /// The 1.8 client always sends two `UseEntity` packets (`InteractAt` then `Interact`) for a
    /// single right-click on an entity, so without this the item would fire twice per click.
    pub last_entity_interact_tick: u64,

    /// World tick of the last `PlayerBlockPlacement` packet that fired a right-click ability
    /// (Etherwarp/Instant Transmission, Ender Pearl, Wither Impact, ...) while a block was within
    /// normal reach. Real vanilla 1.8 sends *two* `PlayerBlockPlacement` packets for one physical
    /// right-click whenever a block is in reach but the held item has no server-registered
    /// "use on block" behavior (which none of these items do): one targeting the real block, then
    /// a fallback packet - confirmed live via `[RECONCILE-DEBUG]`/`[ETHERWARP-DEBUG]` logging,
    /// which showed both `server_teleport` calls for a single click landing in the *same* world
    /// tick, each computed from the position the other had just set - i.e. one click producing
    /// two stacked one-tile hops, which is exactly "sends me one more over." Same root cause and
    /// same fix shape as `last_entity_interact_tick` above (that one guards the analogous
    /// two-packets-per-click quirk for `UseEntity`), just for this different packet type.
    pub last_block_ability_tick: u64,

    // Lava bounce tracking
    pub in_lava: bool,
    pub lava_bounce_last_tick: u64,
    pub lava_bounce_enabled: bool,
    
    // Dungeon stats
    pub dungeon_stats: DungeonPlayerStats,
    
    // Current room tracking for dynamic secrets display
    pub current_room_index: Option<usize>,
    
    // Redstone key tracking (stored on player but not in inventory)
    pub has_redstone_key: bool,

    // Wither key tracking (stored on player but not in inventory) - consumed on opening a Wither Door
    pub has_wither_key: bool,

    // Blood key tracking (stored on player but not in inventory) - consumed on opening a Blood Door
    pub has_blood_key: bool,

    // Track when player first entered the dungeon (for simplified scoreboard)
    pub dungeon_entry_tick: Option<u64>,

    /// Whether the "Undersized party!" head in the CNC menu has already been clicked this menu
    /// session - it's single-use per opening, reset to `false` each time `/cnc` (re)opens the
    /// menu (see `Cnc::run`), not per-player-lifetime.
    pub cnc_undersized_used: bool,

    /// Position/yaw/pitch this player last spawned at in the current practice room (set by
    /// `/practice`, reused as-is by `/rs`) - see `dungeon::practice`. Unused outside practice
    /// mode.
    pub practice_last_spawn: Option<(DVec3, f32, f32)>,

    /// Chunk coordinates still owed to this player from their last `sync_player_view` call
    /// (initial join, or a full dungeon resync), nearest-first, drained a handful per tick by
    /// `main.rs`'s tick loop instead of all at once. See `sync_player_view`'s doc comment for
    /// why: sending the *entire* view distance's worth of `ChunkData` (169 chunks at
    /// `VIEW_DISTANCE=6`) in one synchronous burst overwhelms the vanilla 1.8 client's chunk
    /// mesh-building queue, and any entity spawn packet for a chunk whose render mesh hasn't
    /// been built yet renders as invisible until a full reload (F3+A) forces every chunk to
    /// rebuild at once - "sometimes entities need F3+A to show, like arrows, if I join and
    /// shoot the bow" is exactly that race, since a fired arrow's spawn packet can easily land
    /// in the same burst as (or right after) a hundred-plus still-unmeshed chunks.
    pub pending_chunk_sync: Vec<(i32, i32)>,

    /// Set by `server_teleport`, cleared once the client's exact echo arrives - see
    /// `PendingTeleport`'s own doc comment for the full mechanism.
    pending_teleport: Option<PendingTeleport>,
}

/// Tracks a server-initiated position change (Etherwarp, Instant Transmission, an Ender Pearl
/// landing, Wither Impact, a teleport pad, `/practice`, a dungeon resync, ...) until the client
/// has verifiably applied it, so that a position report the client had already queued *before*
/// receiving the teleport can't silently roll `Player::position` back to where they used to be.
///
/// Real vanilla 1.8 has no dedicated teleport-confirm packet, but it doesn't need a heuristic
/// either: `NetHandlerPlayClient.handlePlayerPosLook` (the client's handler for our `PositionLook`)
/// calls `thePlayer.setPositionAndRotation(...)` with the exact resolved coordinates, and in the
/// very same method, synchronously and unconditionally, sends back a `C06PacketPlayerPosLook` -
/// the combined Player Position And Look packet, `PlayerPositionLook` on this project's own
/// serverbound side - built from `thePlayer.posX`/`getEntityBoundingBox().minY`/`thePlayer.posZ`,
/// i.e. the values it just applied. There is no code path in the real client that processes a
/// server teleport without immediately echoing it back this way, and TCP's ordered delivery on
/// the single per-client connection means every position-bearing packet the client had already
/// queued before receiving our teleport is guaranteed to arrive *before* that echo. So: the first
/// `PlayerPositionLook` whose position exactly matches the destination we most recently sent is
/// unambiguous proof the client is caught up - and nothing else is, no matter how long it's been
/// or how close it happens to look. See `Player::reconcile_position_look_report` for the other
/// half of this (bare `PlayerPosition` reports, which real vanilla never sends as this echo, are
/// rejected outright while a teleport is pending, without even comparing coordinates).
#[derive(Debug)]
struct PendingTeleport {
    /// The destination of the most recent `server_teleport` call - not the whole chain's history,
    /// just the newest one. A newer teleport always replaces this outright (see `server_teleport`),
    /// so an echo matching an *older* teleport in a rapid chain will simply never match this and
    /// gets rejected without disturbing it - there's nothing to reconcile against older history.
    expected_pos: DVec3,
    /// `world.tick_count` when this destination was (most recently) issued - used only for the
    /// diagnostic staleness warning, never to widen acceptance or force an accept.
    sent_tick: u64,
    /// Whether the staleness warning has already been logged for this pending teleport, so it
    /// doesn't spam once every rejected packet past the threshold.
    warned_stale: bool,
}

/// Result of checking one `PlayerPositionLook` report's position against whatever teleport (if
/// any) is currently pending - see `evaluate_position_look_report`.
#[derive(Debug, PartialEq, Eq)]
enum PositionLookOutcome {
    /// No teleport was pending at all - this isn't a reconciliation case, just ordinary movement.
    NoPending,
    /// Matches the newest `expected_pos` within `TELEPORT_RECONCILE_EPSILON` - the real client
    /// echo, or (once pending is `None`) ordinary movement.
    Accepted,
    /// Doesn't match (or isn't finite) - reject, leave `pending_teleport` untouched.
    Rejected,
}

/// Pure decision logic for a `PlayerPositionLook` report, kept free of `Player`/`World` entirely
/// so it can be unit-tested directly (see the `tests` module below) without needing a live
/// server. `Player::reconcile_position_look_report` is a thin wrapper that applies whatever this
/// decides.
fn evaluate_position_look_report(pending: Option<&PendingTeleport>, x: f64, y: f64, z: f64) -> PositionLookOutcome {
    let Some(pending) = pending else {
        return PositionLookOutcome::NoPending;
    };

    // Explicit, not just relying on the comparison below incidentally failing for NaN/inf via
    // IEEE 754 semantics - a future edit to the comparison shouldn't silently lose this.
    if !x.is_finite() || !y.is_finite() || !z.is_finite() {
        return PositionLookOutcome::Rejected;
    }

    let dx = x - pending.expected_pos.x;
    let dy = y - pending.expected_pos.y;
    let dz = z - pending.expected_pos.z;
    let dist_sq = dx * dx + dy * dy + dz * dz;

    if dist_sq <= TELEPORT_RECONCILE_EPSILON * TELEPORT_RECONCILE_EPSILON {
        PositionLookOutcome::Accepted
    } else {
        PositionLookOutcome::Rejected
    }
}

/// Pure decision logic for a bare `PlayerPosition` report - real vanilla's teleport echo is
/// always the combined `PlayerPositionLook` (see `PendingTeleport`'s doc comment), never this, so
/// a bare position report can never be the acknowledgement: reject unconditionally while any
/// teleport is pending, regardless of what coordinates it carries.
fn bare_position_report_rejected(pending: Option<&PendingTeleport>) -> bool {
    pending.is_some()
}

/// Exact-match tolerance for the `PlayerPositionLook` echo, in blocks (squared for the actual
/// comparison - see `reconcile_position_look_report`). Both this server's `PositionLook` and
/// `PlayerPositionLook` structs carry `x`/`y`/`z` as plain `f64` with no fixed-point or truncated
/// encoding (checked directly), and real vanilla's echo is built from the same doubles
/// `setPositionAndRotation` just applied with no lossy detour - so this is a correctness margin
/// for ordinary floating-point noise, not a "how far could they have moved" budget. It must stay
/// far below the smallest realistic single-tick movement (~0.1-0.3 blocks) or a stale report could
/// slip through; it must stay far above float rounding noise (~1e-12) or a legitimate echo could
/// be rejected.
const TELEPORT_RECONCILE_EPSILON: f64 = 1e-3;

/// How long (in ticks) a teleport can sit unacknowledged before logging a one-time diagnostic
/// warning. Purely informational - see `server_teleport`'s doc comment for why there's no
/// accompanying resend or forced-accept: TCP already guarantees the echo arrives eventually on a
/// live connection, and this system would rather wait than ever trust an unverified position.
const TELEPORT_STALE_WARN_TICKS: u64 = 100; // 5 seconds at 20 TPS

impl Player {
    
    pub fn new(
        server: &mut Server,
        client_id: ClientId,
        profile: GameProfile,
        position: DVec3,
        yaw: f32,
        pitch: f32,
    ) -> Self {
        Self {
            server,
            packet_buffer: PacketBuffer::new(),
            network_tx: server.network_tx.clone(),
            profile,
            client_id,
            entity_id: server.world.new_entity_id(),

            position,
            on_ground: false,
            yaw,
            pitch,
            // Must match `position`/`yaw`/`pitch` above, not a hardcoded zero: main.rs's
            // per-tick view-diff compares `position` against `last_position` to compute which
            // chunks are newly in view. A mismatched last_position here makes literally every
            // chunk within view distance look "new" on the player's very first tick, which
            // redundantly re-sends every entity's spawn packets a second time - right after
            // the join handler above already sent them once via `for_each_in_view`. For
            // player-model entities (Mort, NPC mobs) that means a duplicate PlayerListItem +
            // CREATE_TEAM + SpawnPlayer sequence to the same client, which is what was causing
            // broken rendering/interaction (not just occasional invisibility).
            last_position: position,
            last_yaw: yaw,
            last_pitch: pitch,

            ticks_existed: 0,
            last_keep_alive: -1,
            ping: -1,
            is_sneaking: false,

            inventory: Inventory::empty(),
            held_slot: 0,
            
            window_id: 1,
            current_ui: UI::None,
            current_terminal: None,

            sidebar: Scoreboard::new(),
            
            // Bonzo Staff cooldown tracking
            bonzo_last_shot_tick: 0,
            hyperion_last_used: None,
            last_entity_interact_tick: 0,
            last_block_ability_tick: 0,
            
            // Jerry-Chine Gun cooldown tracking
            jerry_last_shot_tick: 0,
            terminator_last_shot_tick: 0,

            // Lava bounce tracking
            in_lava: false,
            lava_bounce_last_tick: 0,
            lava_bounce_enabled: true, // Enable lava bounce by default
            
            // Dungeon stats
            dungeon_stats: DungeonPlayerStats::default(),
            current_room_index: None,
            
            // Redstone key tracking
            has_redstone_key: false,
            has_wither_key: false,
            has_blood_key: false,
            
            // Dungeon entry tracking
            dungeon_entry_tick: None,

            cnc_undersized_used: false,
            practice_last_spawn: None,

            pending_chunk_sync: Vec::new(),
            pending_teleport: None,

            // observed_entities: HashSet::new(),
        }
    }

    /// gets a reference to the server
    pub fn server_mut<'a>(&self) -> &'a mut Server {
        unsafe { self.server.as_mut().expect("Server is null") }
    }

    /// gets a reference to the world
    pub fn world_mut<'a>(&self) -> &'a mut World {
        &mut self.server_mut().world
    }
    
    pub fn write_packet<P : IdentifiedPacket + PacketSerializable>(&mut self, packet: &P) {
        self.packet_buffer.write_packet(packet);
    }
    
    pub fn flush_packets(&mut self) {
        if !self.packet_buffer.buffer.is_empty() {
            let _ = self.network_tx.send(self.packet_buffer.get_packet_message(&self.client_id));
        }
    }
    
    // todo: tick function here?, 
    // pub fn tick(&mut self) -> anyhow::Result<()> {
    //     self.ticks_existed += 1;
    //     self.send_packet(ConfirmTransaction::new())?;
    //     Ok(())
    // }
    
    /// updates player position
    pub fn set_position(&mut self, x: f64, y: f64, z: f64) {
        // A NaN/infinite coordinate here would corrupt every distance/block-lookup that reads
        // `position` afterward - reject outright rather than let it in. The lone caller with any
        // real risk of this is `reconcile_position_look_report` (an untrusted client packet);
        // every other caller in this codebase only ever passes values it derived from its own
        // arithmetic, so this is a no-op for them.
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return;
        }

        // self.last_position = self.position;
        self.position = DVec3::new(x, y, z);

        // Check for falling blocks collision
        self.check_fallingblocks_collision();

        // Check for lava bounce
        // self.check_lava_bounce();
    }

    /// The one place every *server-initiated* position change should go through - Etherwarp,
    /// Instant Transmission, an Ender Pearl landing, Wither Impact, a teleport pad, `/practice`,
    /// a dungeon resync, and so on. Updates the server's own authoritative `position`
    /// synchronously, before the client has any chance to say anything about it - then arms
    /// `pending_teleport` so a position report the client already had queued from before this
    /// teleport can't roll `position` back. See `PendingTeleport`'s doc comment for the full
    /// mechanism.
    ///
    /// Deliberately does *not* touch `last_position` here. `main.rs`'s per-tick loop computes
    /// which chunks are newly in view by diffing `position` against `last_position` (see
    /// `ChunkDiff`/`for_each_diff`) - that comparison is what actually sends `ChunkData` for a
    /// destination the client doesn't already have and unloads chunks that fall out of view.
    /// Setting `last_position = pos` here would make that diff see zero movement (both sides of
    /// the comparison equal), permanently suppressing it for this jump - a real, previously-
    /// shipped bug: a teleport across a chunk boundary left the destination area completely
    /// unsent (not just unrendered - an actual chunk-shaped hole a player would fall through),
    /// unrecoverable by any client-side reload since the data was never sent, and only "fixed"
    /// by walking away and back because *that* produces a genuine `position != last_position` for
    /// one tick. Leaving `last_position` at wherever it was at the start of this tick makes a
    /// teleport look to that diff exactly like an ordinary (if very fast) multi-chunk move -
    /// which is already handled correctly, so nothing needs to be duplicated or invoked
    /// explicitly here. `main.rs`'s own end-of-tick `last_position = position` sync (unconditional,
    /// every tick, every player) catches `last_position` back up afterward, same as it does for
    /// real movement.
    ///
    /// A teleport fired again before the previous one is acknowledged simply replaces
    /// `pending_teleport` with its own (newer) destination - `position` is already correct for
    /// ray/reach calculations the instant this returns, regardless of what the client has or
    /// hasn't caught up to yet. `last_position` still isn't touched by the second call either, so
    /// the diff later this tick correctly reflects the *net* move for the whole tick (start-of-tick
    /// position to the final destination), exactly like it would for any other multi-step move
    /// packed into one tick.
    pub fn server_teleport(&mut self, pos: DVec3, yaw: f32, pitch: f32, flags: u8) {
        self.position = pos;

        // `set_position` normally does this right after updating `position`, but a server
        // teleport bypasses `set_position` entirely (see below). Without this, landing on a
        // falling-floor tile via Etherwarp/teleport wouldn't arm it until the client's next
        // `PlayerPositionLook` echo reconciles - which can be delayed indefinitely if the
        // client's yaw/pitch haven't changed, since bare `Player Position` packets are rejected
        // outright while a teleport is pending. That made the floor feel randomly solid.
        self.check_fallingblocks_collision();

        let tick = self.world_mut().tick_count;
        self.pending_teleport = Some(PendingTeleport {
            expected_pos: pos,
            sent_tick: tick,
            warned_stale: false,
        });

        self.write_packet(&crate::net::protocol::play::clientbound::PositionLook {
            x: pos.x,
            y: pos.y,
            z: pos.z,
            yaw,
            pitch,
            flags,
        });
    }

    /// `PlayerPosition` (bare position, no rotation) handler's entry point. Real vanilla's
    /// `handlePlayerPosLook` only ever answers a server teleport with the *combined*
    /// `C06PacketPlayerPosLook` (see `PendingTeleport`'s doc comment) - never a bare position
    /// packet - so while a teleport is pending, a bare position report cannot possibly be that
    /// acknowledgement. It's rejected outright, without even looking at its coordinates: either
    /// it's a report the client queued before receiving the teleport (stale), or it's a real
    /// post-teleport movement tick that hasn't also changed look (which can only happen *after*
    /// the real echo already arrived and cleared `pending_teleport` - so this branch is never
    /// reached for it in the first place).
    pub fn reconcile_bare_position_report(&mut self, x: f64, y: f64, z: f64) {
        if bare_position_report_rejected(self.pending_teleport.as_ref()) {
            self.warn_if_teleport_stale();
            return;
        }
        self.set_position(x, y, z);
    }

    /// `PlayerPositionLook` handler's entry point. Rotation is applied unconditionally either
    /// way - it was never part of the race this guards against, and there's no reason a rejected
    /// position should also swallow a legitimate look update. Position is applied, and
    /// `pending_teleport` cleared, only when it lands within `TELEPORT_RECONCILE_EPSILON` of the
    /// *newest* teleport this server has issued - see `PendingTeleport`'s doc comment for why
    /// that's a real proof of acknowledgement here, not a heuristic. Anything else - including an
    /// echo for an older teleport in a rapid chain, which will simply never match the newest
    /// `expected_pos` - is rejected and `pending_teleport` is left untouched.
    pub fn reconcile_position_look_report(&mut self, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) {
        self.yaw = yaw;
        self.pitch = pitch;

        match evaluate_position_look_report(self.pending_teleport.as_ref(), x, y, z) {
            PositionLookOutcome::NoPending | PositionLookOutcome::Accepted => {
                self.set_position(x, y, z);
                self.pending_teleport = None;
            }
            PositionLookOutcome::Rejected => {
                self.warn_if_teleport_stale();
            }
        }
    }

    /// Logs a one-time diagnostic if `pending_teleport` has gone unacknowledged for an unusually
    /// long time - see `TELEPORT_STALE_WARN_TICKS`'s doc comment for why this only logs, it
    /// never resends or force-accepts anything.
    fn warn_if_teleport_stale(&mut self) {
        let now = self.world_mut().tick_count;
        let client_id = self.client_id;
        if let Some(pending) = self.pending_teleport.as_mut() {
            if !pending.warned_stale && now.saturating_sub(pending.sent_tick) >= TELEPORT_STALE_WARN_TICKS {
                eprintln!(
                    "[teleport] client {client_id} hasn't acknowledged a server teleport after {} ticks (still waiting for the exact PlayerPositionLook echo)",
                    now.saturating_sub(pending.sent_tick)
                );
                pending.warned_stale = true;
            }
        }
    }

    // /// Check for lava bounce when player enters lava
    // fn check_lava_bounce(&mut self) {
    //     use crate::server::block::blocks::Blocks;
    //     use crate::net::protocol::play::clientbound::EntityVelocity;
    //     use crate::net::var_int::VarInt;
    //     
    //     let world = self.world_mut();
    //     let block_pos = DVec3::new(
    //         self.position.x.floor(),
    //         self.position.y.floor(),
    //         self.position.z.floor(),
    //     );
    //     
    //     // Check if player's feet are touching lava surface (at floor level)
    //     let feet_block = world.get_block_at(block_pos.x as i32, block_pos.y as i32, block_pos.z as i32);
    //     let in_lava = match feet_block {
    //         Blocks::Lava { .. } | Blocks::FlowingLava { .. } => true,
    //         _ => false,
    //     };
    //     
    //     let was_in_lava = self.in_lava;
    //     self.in_lava = in_lava;
    //     
    //     // Only bounce when entering lava (not already in lava) and if enabled
    //     if in_lava && !was_in_lava && self.lava_bounce_enabled {
    //         let current_tick = world.tick_count;
    //         const LAVA_BOUNCE_COOLDOWN_TICKS: u64 = 10; // 500ms cooldown like Java
    //         
    //         // Use a shorter cooldown for 0 ping to prevent double bouncing
    //         const SHORT_BOUNCE_COOLDOWN_TICKS: u64 = 5; // 250ms for 0 ping scenarios
    //         
    //         if current_tick - self.lava_bounce_last_tick >= SHORT_BOUNCE_COOLDOWN_TICKS {
    //             
    //             // Apply upward velocity for lava bounce
    //             const LAVA_BOUNCE_VELOCITY: f64 = 0.5; // Upward velocity for instant surface bounce
    //             
    //             self.write_packet(&EntityVelocity {
    //                 entity_id: VarInt(self.entity_id),
    //                 velocity_x: (0.0 * 8000.0) as i16, // Keep horizontal motion
    //                 velocity_y: (LAVA_BOUNCE_VELOCITY * 8000.0) as i16, // Upward bounce
    //                 velocity_z: (0.0 * 8000.0) as i16, // Keep horizontal motion
    //             });
    //             
    //             self.lava_bounce_last_tick = current_tick;
    //             
    //         }
    //     }
    // }
    
    pub fn collision_aabb(&self) -> AABB {
        let w = 0.3;
        let h = 1.8;
        AABB::new(
            DVec3::new(self.position.x - w, self.position.y, self.position.z - w),
            DVec3::new(self.position.x + w, self.position.y + h, self.position.z + w),
        )
    }

    /// Check for falling blocks collision when player moves
    fn check_fallingblocks_collision(&mut self) {
        let server = self.server_mut();
        let world = &mut server.world;
        let dungeon = &mut server.dungeon;

        // `as i32` truncates toward zero, not toward negative infinity - fine for positive
        // coordinates (where truncation and flooring agree) but wrong for the negative ones this
        // dungeon actually uses: e.g. `(-161.7_f64) as i32 == -161`, even though the block cell
        // that world position is actually inside of is `-162` (cells span `[-162.0, -161.0)`).
        // Every continuous movement update has some nonzero fractional part, so on a negative
        // coordinate this was shifting the computed cell one step toward zero almost every time.
        // A pattern tile in the interior of a contiguous falling-floor pattern could still
        // accidentally match (the wrongly-shifted cell often lands on another tile that's *also*
        // in the same pattern), but a tile at the pattern's edge - e.g. the row against a wall,
        // with no neighbouring pattern tile on the shifted-into side - had nothing to accidentally
        // match, so the trigger silently never fired there. `.floor()` gives the true cell either
        // way.
        let feet_x = self.position.x.floor() as i32;
        let feet_z = self.position.z.floor() as i32;

        // Find the room the player is in
        if let Some(room_index) = dungeon.get_room_at(feet_x, feet_z) {
            let room = dungeon.rooms.get_mut(room_index).unwrap();
            // Resolve the block actually supporting the player's feet, not just "one cell below
            // a truncated Y". A full block's top surface sits at an exact integer height, so
            // flooring position.y and subtracting 1 works there - but a bottom-half slab's
            // surface sits at `y + 0.5`, which floors down to `y` itself; subtracting 1 would
            // then miss the slab and check the empty cell underneath it. Subtracting a tiny
            // epsilon before flooring maps an exact-integer feet height down into the cell below
            // (matching the full-block case) while still mapping a `y + 0.5` feet height onto
            // its own cell (matching the slab case).
            let feet_block_pos = crate::server::block::block_position::BlockPos {
                x: feet_x,
                y: (self.position.y - 1e-4).floor() as i32,
                z: feet_z,
            };

            // Check for falling blocks collision
            room.check_fallingblocks_collision(world, room_index, &feet_block_pos);
        }
    }

    pub fn handle_left_click(&mut self) {
        
    }
    
    pub fn handle_right_click(&mut self) {
        if let Some(ItemSlot::Filled(item, _)) = self.inventory.get_hotbar_slot(self.held_slot as usize) {
            item.on_right_click(self).unwrap()
        }
    }

    /// Returns the world-space position of the player's eyes.
    pub fn player_eye_position(&self) -> DVec3 {
        let mut position = self.position;
        position.y += 1.62;
        position
    }

    /// Returns the forward look direction vector from the player's yaw/pitch.
    pub fn rotation_vec(&self) -> DVec3 {
        let yaw_rad = (self.yaw as f64).to_radians();
        let pitch_rad = (self.pitch as f64).to_radians();
        let (yaw_sin, yaw_cos) = (yaw_rad.sin(), yaw_rad.cos());
        DVec3::new(
            -pitch_rad.cos() * yaw_sin,
            -pitch_rad.sin(),
            pitch_rad.cos() * yaw_cos,
        )
    }
    
    /// Shoot Bonzo projectile with cooldown and delay like the Java version
    pub fn shoot_bonzo_projectile(&mut self) -> anyhow::Result<()> {
        use crate::server::items::bonzo_projectile::BonzoProjectileImpl;
        use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
        use crate::server::utils::sounds::Sounds;
        use crate::net::protocol::play::clientbound::SoundEffect;
        
        let current_tick = self.world_mut().tick_count;
        
        // Bonzo Staff cooldown (equivalent to bonzoCd in Java - 500ms = 10 ticks)
        const BONZO_COOLDOWN_TICKS: u64 = 3; // 150ms = 3 ticks at 20 TPS
        if current_tick - self.bonzo_last_shot_tick < BONZO_COOLDOWN_TICKS {
            return Ok(()); // Silently ignore if on cooldown
        }
        
        // Play ghast moan sound immediately (like Java version)
        self.write_packet(&SoundEffect {
            sound: Sounds::GhastMoan.id(),
            pos_x: self.position.x,
            pos_y: self.position.y,
            pos_z: self.position.z,
            volume: 1.0,
            pitch: 1.43,
        });
        
        // Calculate spawn position with forward offset
        let eye_height = 1.62; // Player eye height
        let yaw_rad = (self.yaw as f64).to_radians();
        let pitch_rad = (self.pitch as f64).to_radians();
        
        let direction = DVec3::new(
            -pitch_rad.cos() * yaw_rad.sin(),
            -pitch_rad.sin(),
            pitch_rad.cos() * yaw_rad.cos(),
        ).normalize();
        
        let spawn_offset = 0.15; // Small forward offset like Java version
        let spawn_pos = DVec3::new(
            self.position.x + direction.x * spawn_offset,
            self.position.y + eye_height + direction.y * spawn_offset,
            self.position.z + direction.z * spawn_offset,
        );
        
        // Spawn projectile immediately with full velocity (like Java version)
        let _projectile_id = self.world_mut().spawn_entity(
            spawn_pos,
            EntityMetadata::new(EntityVariant::BonzoProjectile),
            BonzoProjectileImpl::new(self.client_id, direction, 20.0), // Full speed immediately
        )?;
        
        // Update cooldown
        self.bonzo_last_shot_tick = current_tick;
        
        Ok(())
    }
    
    /// Shoot Jerry-Chine Gun projectile with cooldown and random spread like the Java version
    pub fn shoot_jerry_projectile(&mut self) -> anyhow::Result<()> {
        use crate::server::items::jerry_projectile::JerryProjectileImpl;
        use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
        
        let current_tick = self.world_mut().tick_count;
        
        // Jerry-Chine Gun cooldown (40ms = 0.8 ticks, round up to 1 tick)
        const JERRY_COOLDOWN_TICKS: u64 = 1;
        if current_tick - self.jerry_last_shot_tick < JERRY_COOLDOWN_TICKS {
            return Ok(()); // Silently ignore if on cooldown
        }
        
        // Calculate spawn position with forward offset and random spread
        let eye_height = 1.62; // Player eye height
        
        // Add random spread: yaw ±10 degrees, pitch ±10 degrees (like Java: rand.nextInt(21) - 10)
        let yaw_spread = rand::random::<f32>() * 21.0 - 10.0;
        let pitch_spread = rand::random::<f32>() * 21.0 - 10.0;
        
        let yaw_rad = ((self.yaw + yaw_spread) as f64).to_radians();
        let pitch_rad = ((self.pitch + pitch_spread) as f64).to_radians();
        
        let direction = DVec3::new(
            -pitch_rad.cos() * yaw_rad.sin(),
            -pitch_rad.sin(),
            pitch_rad.cos() * yaw_rad.cos(),
        ).normalize();
        
        let spawn_offset = 0.5; // Like Java version: 0.5D
        let spawn_pos = DVec3::new(
            self.position.x + direction.x * spawn_offset,
            self.position.y + eye_height + direction.y * spawn_offset,
            self.position.z + direction.z * spawn_offset,
        );
        
        // Spawn projectile immediately with full velocity (like Java version)
        let _projectile_id = self.world_mut().spawn_entity(
            spawn_pos,
            EntityMetadata::new(EntityVariant::JerryProjectile),
            JerryProjectileImpl::new(self.client_id, direction, 20.0), // Same speed as Bonzo
        )?;
        
        // Update cooldown
        self.jerry_last_shot_tick = current_tick;
        
        Ok(())
    }
    
    pub fn open_ui(&mut self, ui: UI) {
        self.current_ui = ui;
        // kind of temporary solution,
        // instead of just putting the item in an available slot if it is dragged
        if ui == UI::Inventory { 
            if let ItemSlot::Filled(item, _) = self.inventory.dragged_item { 
                self.write_packet(&SetSlot {
                    window_id: -1,
                    slot: 0,
                    item_stack: Some(item.get_item_stack()),
                })
            }
        }
        if let Some(container_data) = ui.get_container_data() {
            // team we forgot to check for wId exceeding 100
            if self.window_id > 99 {
                self.window_id = 1;
            } else {
                self.window_id += 1;
            }
            self.write_packet(&OpenWindow {
                window_id: self.window_id,
                inventory_type: "minecraft:container".into(),
                window_title: ChatComponentTextBuilder::new(container_data.title).build(),
                slot_count: container_data.slot_amount,
            });
            self.sync_inventory();
        }
    }
    
    pub fn sync_inventory(&mut self) {
        let mut inv_items = Vec::new();
        
        for item in &self.inventory.items {
            inv_items.push(item.get_item_stack())
        }
        
        if let Some(items) = self.current_ui.get_container_contents(self.server_mut(), &self.client_id)  {
            for i in 0..items.len() {
                self.write_packet(&SetSlot {
                    window_id: self.window_id,
                    slot: i as i16,
                    item_stack: items.get(i).unwrap_or_else(|| &None).clone(),
                });
            }
        }
        
        self.write_packet(&WindowItems {
            window_id: 0,
            items: inv_items,
        });
        if let UI::Inventory = self.current_ui {
            self.write_packet(&SetSlot {
                window_id: -1,
                slot: 0,
                item_stack: self.inventory.dragged_item.get_item_stack(),
            })
        } else {
            self.write_packet(&SetSlot {
                window_id: -1,
                slot: 0,
                item_stack: None,
            })
        }
    }
    
    pub fn send_message(&mut self, msg: &str) {
        self.write_packet(&Chat {
            component: ChatComponentTextBuilder::new(msg).build(),
            chat_type: 0 
        })
    }
    
    pub fn send_action_bar(&mut self, legacy_text: &str) {
        use crate::server::player::dungeon_stats::legacy_to_chat_component;
        self.write_packet(&Chat {
            component: legacy_to_chat_component(legacy_text),
            chat_type: 2, // Position 2 = action bar
        })
    }

}

/// Unit tests for the teleport-reconciliation decision logic (`evaluate_position_look_report`,
/// `bare_position_report_rejected`). Deliberately exercise the pure functions directly rather
/// than going through `Player`/`server_teleport` - those need a live `World`/`Server` only to
/// write packets and read `tick_count` for the diagnostic warning, neither of which the actual
/// accept/reject decision depends on. Each "issue a teleport" step below is simulated inline
/// (constructing a `PendingTeleport` the same way `server_teleport` does) so these tests describe
/// full multi-teleport timelines without any server plumbing.
#[cfg(test)]
mod teleport_reconciliation_tests {
    use super::*;

    fn pending_at(pos: DVec3, sent_tick: u64) -> PendingTeleport {
        PendingTeleport { expected_pos: pos, sent_tick, warned_stale: false }
    }

    /// One teleport, then its exact echo: accepted, pending cleared.
    #[test]
    fn one_teleport_exact_echo_accepted() {
        let b = DVec3::new(100.0, 70.0, 100.0);
        let pending = pending_at(b, 0);

        let outcome = evaluate_position_look_report(Some(&pending), b.x, b.y, b.z);
        assert_eq!(outcome, PositionLookOutcome::Accepted);
    }

    /// The correct newest C06 acknowledgement, arriving as the very next packet: accepted.
    /// (Same shape as the above, kept as its own test since it's an explicitly requested case -
    /// the "happy path" every other test's rejections are contrasted against.)
    #[test]
    fn newest_c06_acknowledgement_is_accepted() {
        let dest = DVec3::new(-42.25, 68.0, 17.75);
        let pending = pending_at(dest, 12);

        assert_eq!(
            evaluate_position_look_report(Some(&pending), dest.x, dest.y, dest.z),
            PositionLookOutcome::Accepted
        );
    }

    /// Rapid forward teleports (A -> B -> C, all in the same general direction). A stale report
    /// still describing A must be rejected against the newest destination (C), and must not be
    /// satisfiable by coincidence - then C's own echo is accepted.
    #[test]
    fn rapid_forward_teleports_reject_stale_then_accept_newest() {
        let a = DVec3::new(0.0, 70.0, 0.0);
        let b = DVec3::new(20.0, 70.0, 0.0);
        let c = DVec3::new(40.0, 70.0, 0.0);

        // teleport 1 fires (A -> B), teleport 2 fires immediately after (B -> C) - pending now
        // only remembers C, exactly like `server_teleport` overwriting `pending_teleport`.
        let pending = pending_at(c, 1);

        // A stale packet describing the client's real pre-teleport-1 position.
        assert_eq!(
            evaluate_position_look_report(Some(&pending), a.x, a.y, a.z),
            PositionLookOutcome::Rejected
        );
        // The genuine echo for C.
        assert_eq!(
            evaluate_position_look_report(Some(&pending), c.x, c.y, c.z),
            PositionLookOutcome::Accepted
        );
    }

    /// Reversed teleport: B -> A' where A' is very close to (here, exactly) the original starting
    /// point A. The newest destination is what matters, not the direction or history - a report
    /// matching A' must be accepted purely because it matches the *newest* `expected_pos`, with
    /// no comparison to A ever entering into it.
    #[test]
    fn reversed_teleport_back_near_origin_is_judged_only_against_newest() {
        let a = DVec3::new(5.0, 70.0, 5.0);
        let b = DVec3::new(500.0, 70.0, 500.0);
        // Teleport 2 lands back exactly on the original position.
        let pending = pending_at(a, 5);

        assert_eq!(
            evaluate_position_look_report(Some(&pending), a.x, a.y, a.z),
            PositionLookOutcome::Accepted
        );
        // A report still describing the intermediate destination B is unrelated to the newest
        // expected position and must be rejected.
        assert_eq!(
            evaluate_position_look_report(Some(&pending), b.x, b.y, b.z),
            PositionLookOutcome::Rejected
        );
    }

    /// An older teleport's echo, arriving *after* a newer teleport has already been issued, must
    /// not be accepted and must not clear the pending state that's tracking the newer one.
    #[test]
    fn older_echo_after_newer_teleport_does_not_clear_pending() {
        let b = DVec3::new(10.0, 70.0, -10.0);
        let c = DVec3::new(-30.0, 65.0, 200.0);
        // As in `rapid_forward_teleports`: after both teleports fire, only the newest (C)
        // survives in `pending_teleport` - there is nothing "older" left to accidentally clear
        // from, which is itself the point: b's own echo simply cannot match c.
        let pending = pending_at(c, 3);

        let outcome = evaluate_position_look_report(Some(&pending), b.x, b.y, b.z);
        assert_eq!(outcome, PositionLookOutcome::Rejected);
        // Confirm the pending state a real caller would still be holding is unchanged - the
        // rejection path never touches `pending`, only the caller decides whether to keep it.
        assert_eq!(pending.expected_pos, c);
    }

    /// Stale bare `PlayerPosition` packets (no rotation) must be rejected outright while any
    /// teleport is pending, regardless of what coordinates they carry - even coordinates that
    /// exactly match the pending destination, since real vanilla never sends this packet type as
    /// its teleport echo.
    #[test]
    fn stale_bare_position_rejected_while_pending_even_if_coordinates_match() {
        let dest = DVec3::new(12.0, 70.0, 12.0);
        let pending = pending_at(dest, 0);

        assert!(bare_position_report_rejected(Some(&pending)));

        // Once nothing is pending, bare position reports are ordinary movement again.
        assert!(!bare_position_report_rejected(None));
    }

    /// A `PlayerPositionLook` whose position doesn't match the newest destination at all (not a
    /// stale-history coincidence, just genuinely different coordinates) is rejected.
    #[test]
    fn mismatched_c06_position_rejected() {
        let dest = DVec3::new(0.0, 70.0, 0.0);
        let pending = pending_at(dest, 0);

        let unrelated = DVec3::new(9999.0, 5.0, -1234.5);
        assert_eq!(
            evaluate_position_look_report(Some(&pending), unrelated.x, unrelated.y, unrelated.z),
            PositionLookOutcome::Rejected
        );
    }

    /// Epsilon boundary: just inside accepts, just outside rejects - and non-finite coordinates
    /// are always rejected regardless of distance.
    #[test]
    fn epsilon_boundary_and_non_finite_coordinates() {
        let dest = DVec3::new(0.0, 70.0, 0.0);
        let pending = pending_at(dest, 0);

        let just_inside = TELEPORT_RECONCILE_EPSILON * 0.5;
        assert_eq!(
            evaluate_position_look_report(Some(&pending), just_inside, dest.y, dest.z),
            PositionLookOutcome::Accepted
        );

        let just_outside = TELEPORT_RECONCILE_EPSILON * 2.0;
        assert_eq!(
            evaluate_position_look_report(Some(&pending), just_outside, dest.y, dest.z),
            PositionLookOutcome::Rejected
        );

        assert_eq!(
            evaluate_position_look_report(Some(&pending), f64::NAN, dest.y, dest.z),
            PositionLookOutcome::Rejected
        );
        assert_eq!(
            evaluate_position_look_report(Some(&pending), f64::INFINITY, dest.y, dest.z),
            PositionLookOutcome::Rejected
        );
    }

    /// No teleport pending at all: any report is `NoPending`, i.e. ordinary movement, not subject
    /// to reconciliation.
    #[test]
    fn no_pending_teleport_is_ordinary_movement() {
        let pos = DVec3::new(1.0, 2.0, 3.0);
        assert_eq!(evaluate_position_look_report(None, pos.x, pos.y, pos.z), PositionLookOutcome::NoPending);
        assert!(!bare_position_report_rejected(None));
    }
}
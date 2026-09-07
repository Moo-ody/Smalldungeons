use crate::dungeon::dungeon_state::DungeonState;
use crate::dungeon::room::secrets::DungeonSecret;
use crate::net::protocol::play::clientbound::{BlockAction, Chat, Particles, SoundEffect};
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::NoEntityImpl;
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::Player;
use crate::server::player::inventory::ItemSlot;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::color::MCColors;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::redstone::is_special_lever;
use std::cell::RefCell;
use std::rc::Rc;
// use std::collections::HashMap;

use crate::server::world::ScheduledSound;
use crate::server::world::tactical_insertion::TacticalInsertionMarker;

#[derive(Debug)]
pub enum BlockInteractAction {
    WitherDoor {
        door_index: usize
    },
    BloodDoor {
        door_index: usize
    },
    Chest {
        secret: Rc<RefCell<DungeonSecret>>,
    },
    /// The dungeon's one designated mimic chest (see `main.rs`'s post-locked-chest-spawn
    /// selection step, which swaps a random locked chest's block/interactable entry for this)
    /// - same appearance as a normal chest, block type `TrappedChest` instead of `Chest`. On
    /// open: the chest block disappears and `spawner::spawn_mimic` spawns the mob in its place.
    MimicChest,
    WitherEssence {
        secret: Rc<RefCell<DungeonSecret>>,
    },
    Lever,
    // Mushroom secret: bottom mushrooms (start) and top mushrooms (return nodes)
    MushroomBottom {
        set_index: usize,
    },
    MushroomTop,
    RedstoneKeySkull {
        room_index: usize,
    },
    /// One of the Three Weirdos puzzle's 3 chests - see `dungeon::room::three_weirdos`. `state`
    /// is shared (`Rc<RefCell<_>>`, same idiom as `Self::Chest`'s `secret`) with the 3 NPC
    /// entities' own interact handlers and the other 2 chests, so all 6 see the same
    /// `clicked_weirdos`/`resolved` state.
    ThreeWeirdosChest {
        chest_index: usize,
        state: Rc<RefCell<crate::dungeon::room::three_weirdos::ThreeWeirdosState>>,
    },
    /// One of the Creeper Beams puzzle's 22 sea lantern positions - see
    /// `dungeon::room::creeper_beams`. Triggered either by a direct right-click or (the actual
    /// intended way to play it) a Terminator arrow hitting the block - see
    /// `ai::projectile::MobProjectileImpl`'s `on_block_hit`.
    CreeperBeamsLantern {
        state: Rc<RefCell<crate::dungeon::room::creeper_beams::CreeperBeamsState>>,
    },
    /// One of the Tic Tac Toe puzzle's 9 wall buttons - see `dungeon::room::tic_tac_toe`.
    TicTacToeButton {
        state: Rc<RefCell<crate::dungeon::room::tic_tac_toe::TicTacToeState>>,
        index: usize,
    },
    QuizButton {
        state: Rc<RefCell<crate::dungeon::room::quiz::QuizState>>,
        answer_index: usize,
    },
    /// A button or qualifying face-center sign on one face of a Boulder puzzle box - see
    /// `dungeon::room::boulder`. Pushing it moves the box at `(grid_x, grid_z)` one grid cell in
    /// `direction`.
    BoulderTrigger {
        state: Rc<RefCell<crate::dungeon::room::boulder::BoulderState>>,
        grid_x: i32,
        grid_z: i32,
        direction: crate::server::utils::direction::Direction,
    },
    /// One of the Water Board puzzle's 7 real levers - see `dungeon::room::waterboard`.
    WaterboardLever {
        state: Rc<RefCell<crate::dungeon::room::waterboard::WaterboardState>>,
        lever: crate::dungeon::room::waterboard::LeverKind,
    },
    // mainly for quick debug,
    Callback(fn(&Player, &BlockPos)),
}


impl BlockInteractAction {
    pub fn interact(&self, player: &mut Player, block_pos: &BlockPos) {
        match self {
            Self::WitherDoor { door_index: id } => {
                if !player.has_wither_key {
                    // Reference-server-captured fail behavior (see secrets.rs's "door fail" log):
                    // mob.endermen.portal at volume 8.0, pitch 0.0, plus a red chat line - only
                    // to the player who clicked, not a broadcast like the open message below.
                    let _ = player.write_packet(&SoundEffect {
                        sound: Sounds::EndermenPortal.id(),
                        volume: 8.0,
                        pitch: 0.0,
                        pos_x: block_pos.x as f64,
                        pos_y: block_pos.y as f64,
                        pos_z: block_pos.z as f64,
                    });
                    player.send_message("§cYou do not have the key for this door!");
                    return;
                }

                // Play wither door opening sound effect
                let _ = player.write_packet(&SoundEffect {
                    sound: Sounds::NotePling.id(),
                    volume: 8.0,
                    pitch: 4.05,
                    pos_x: block_pos.x as f64,
                    pos_y: block_pos.y as f64,
                    pos_z: block_pos.z as f64,
                });

                let world = &mut player.server_mut().world;
                let dungeon = &mut player.server_mut().dungeon;

                if let DungeonState::Started { .. } = dungeon.state {
                    player.has_wither_key = false;
                    let door = &dungeon.doors[*id];
                    door.open_door(world);
                    dungeon.doors[*id].opened = true;

                    if let Some(room_index) = player.current_room_index {
                        player.server_mut().dungeon.update_map_for_room(room_index);
                    }

                    // Send message to all players when WITHER door is opened
                    let message = format!("§a{} §aopened a §8§lWITHER §adoor!", player.profile.username);
                    for (_, other_player) in &mut player.server_mut().world.players {
                        let _ = other_player.send_message(&message);
                    }
                }
            }

            Self::BloodDoor { door_index: id } => {
                if !player.has_blood_key {
                    // Same fail behavior as `WitherDoor` above - only to the clicking player.
                    let _ = player.write_packet(&SoundEffect {
                        sound: Sounds::EndermenPortal.id(),
                        volume: 8.0,
                        pitch: 0.0,
                        pos_x: block_pos.x as f64,
                        pos_y: block_pos.y as f64,
                        pos_z: block_pos.z as f64,
                    });
                    player.send_message("§cYou do not have the key for this door!");
                    return;
                }

                // Play blood door opening sound effect (ghast scream)
                let _ = player.write_packet(&SoundEffect {
                    sound: Sounds::GhastScream.id(),
                    volume: 2.0,
                    pitch: 0.49,
                    pos_x: block_pos.x as f64,
                    pos_y: block_pos.y as f64,
                    pos_z: block_pos.z as f64,
                });

                let world = &mut player.server_mut().world;
                let dungeon = &mut player.server_mut().dungeon;

                if let DungeonState::Started { .. } = dungeon.state {
                    player.has_blood_key = false;
                    let door = &dungeon.doors[*id];
                    door.open_door(world);
                    dungeon.doors[*id].opened = true;

                    if let Some(room_index) = player.current_room_index {
                        player.server_mut().dungeon.update_map_for_room(room_index);
                    }

                    // Send message to all players when BLOOD door is opened
                    for (_, other_player) in &mut player.server_mut().world.players {
                        let _ = other_player.send_message("§cThe §c§lBLOOD DOOR §chas been opened!");
                        let _ = other_player.send_message("§5A shiver runs down your spine...");
                    }
                }
            }

            Self::Chest { secret } => {
                // Check if this chest is locked
                let is_locked = {
                    let dungeon = &player.server_mut().dungeon;
                    dungeon.locked_chests.get(block_pos)
                        .map(|chest_state| chest_state.locked)
                        .unwrap_or(false)
                };
                if is_locked {
                    // Chest is locked - send red message and cancel opening
                    player.write_packet(&Chat {
                        component: ChatComponentTextBuilder::new("That chest is locked!")
                            .color(MCColors::Red)
                            .build(),
                        chat_type: 0,
                    });
                    return;
                }

                let mut secret = secret.borrow_mut();
                if !secret.obtained {
                    // maybe make this a packet where it is sent to all players
                    player.write_packet(&BlockAction {
                        block_pos: block_pos.clone(),
                        event_id: 1,
                        event_data: 1,
                        block_id: 54,
                    });
                    player.write_packet(&SoundEffect {
                        sound: "random.chestopen",
                        pos_x: block_pos.x as f64,
                        pos_y: block_pos.y as f64,
                        pos_z: block_pos.z as f64,
                        volume: 1.0,
                        pitch: 0.975,
                    });
                    secret.obtained = true;
                    
                    // Increment found_secrets for the room containing this secret (only if not
                    // already obtained, and only if this chest actually counts as one of the
                    // room's secrets - see `DungeonSecret::counts_as_secret`'s doc comment).
                    if secret.counts_as_secret {
                        if let Some(room_index) = player.server_mut().dungeon.get_room_at(block_pos.x, block_pos.z) {
                            // Trap rooms have no starred mobs at all, so `DungeonMap::draw_room`'s
                            // usual `starred_mobs_remaining == 0` clear check can't apply to them (it
                            // reads true the instant the room is entered - see `room.rs`'s
                            // `trap_completion_chest_pos` doc comment for the full story). This is
                            // that room type's actual clear signal instead: grabbing this one
                            // specific chest.
                            let is_trap_completion_chest = {
                                let room = player.server_mut().dungeon.rooms.get(room_index);
                                room.is_some_and(|r| r.trap_completion_chest_pos == Some(*block_pos))
                            };

                            let should_update_map = {
                                let room = player.server_mut().dungeon.rooms.get(room_index);
                                room.map(|r| (r.found_secrets < r.room_data.secrets && r.entered) || is_trap_completion_chest).unwrap_or(false)
                            };

                            if let Some(room) = player.server_mut().dungeon.rooms.get_mut(room_index) {
                                if room.found_secrets < room.room_data.secrets {
                                    room.found_secrets += 1;
                                }
                                if is_trap_completion_chest {
                                    room.trap_completed = true;
                                }
                            }

                            // Update map if room is entered and secret count changed (or this was the
                            // Trap room's completion chest, which changes its clear state even if
                            // every other secret was already found)
                            if should_update_map {
                                player.server_mut().dungeon.update_map_for_room(room_index);
                            }
                        }
                    }

                    // A chest whose own opening is a puzzle's actual win condition (Teleport
                    // Maze - confirmed: reaching it only reveals the chest, opening it is what
                    // actually solves the puzzle, unlike e.g. Creeper Beams which completes on
                    // its own) marks that room done and broadcasts the solved message right here,
                    // not any earlier.
                    if let Some(puzzle_room_index) = secret.puzzle_room_index {
                        // A single-chest puzzle (`puzzle_chest_group: None`) completes right away,
                        // same as before. A grouped puzzle (Ice Fill's two blessing chests) only
                        // completes once every chest in the group has been opened - this chest's
                        // own open just counts down the group's shared counter.
                        let ready = match &secret.puzzle_chest_group {
                            Some(counter) => {
                                let remaining = counter.get().saturating_sub(1);
                                counter.set(remaining);
                                remaining == 0
                            }
                            None => true,
                        };
                        if ready {
                            let room_name = player.server_mut().dungeon.rooms.get(puzzle_room_index)
                                .map(|r| r.room_data.name.clone())
                                .unwrap_or_default();
                            if let Some(room) = player.server_mut().dungeon.rooms.get_mut(puzzle_room_index) {
                                room.puzzle_completed = true;
                            }
                            let username = player.profile.username.clone();
                            let message = format!("\u{a7}a\u{a7}lPUZZLE SOLVED! \u{a7}a{username} \u{a7}esolved the {room_name} puzzle!");
                            for other_player in player.server_mut().world.players.values_mut() {
                                other_player.send_message(&message);
                            }
                            player.server_mut().dungeon.update_map_for_room(puzzle_room_index);
                        }
                    }

                    // A "blessing" chest also gets the same real captured effect
                    // `WitherEssence` uses (this file's own left-behind note above confirmed it,
                    // once - "if its a blessing, can likely re-use wither essence as these values
                    // appear to be the same"): the ascending `note.harp` sequence, plus a
                    // floating, spinning skull wearing the blessing's own texture.
                    if let Some(texture) = secret.blessing_texture {
                        let world = player.world_mut();
                        let pitches = [0.7936508, 0.8888889, 1.0, 1.0952381, 1.1904762];
                        let world_tick = world.tick_count;
                        let pos = (block_pos.x as f64 + 0.5, block_pos.y as f64 + 0.5, block_pos.z as f64 + 0.5);
                        for (i, &pitch) in pitches.iter().enumerate() {
                            world.scheduled_fixed_sounds.push(crate::server::world::tactical_insertion::ScheduledFixedSound {
                                due_tick: world_tick + (i as u64 * 5),
                                sound: Sounds::Harp,
                                volume: 1.0,
                                pitch,
                                pos_x: pos.0,
                                pos_y: pos.1,
                                pos_z: pos.2,
                            });
                        }

                        use crate::dungeon::blessings::BlessingKind;
                        use crate::dungeon::room::secrets::EssenceEntityImpl;
                        // One of the 4 real Catacombs blessing types, picked fresh each time this
                        // specific chest is opened. The level (and the actual "DUNGEON BUFF!"
                        // chat announcement) isn't decided here - it's computed fresh, from the
                        // dungeon's own persistent per-type level tracker, right when the
                        // essence despawns (`EssenceEntityImpl::tick`'s despawn branch) - matches
                        // real Hypixel blessings being a shared, run-persistent level, not a
                        // fresh roll each pickup.
                        use rand::Rng;
                        let blessing_type = BlessingKind::ALL[rand::rng().random_range(0..BlessingKind::ALL.len())];
                        let nametag = format!("\u{a7}dBlessing of {}", blessing_type.display_name());

                        // Same offset `WitherEssence` uses for this exact effect - the armor
                        // stand's own body height means the equipped head only lines up with
                        // where the block was at -1.4 on Y.
                        let _ = world.spawn_entity(
                            DVec3::from(block_pos).add_x(0.5).add_y(-1.4).add_z(0.5),
                            {
                                let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
                                metadata.is_invisible = true;
                                metadata.custom_name = Some(nametag);
                                metadata.custom_name_visible = true;
                                metadata
                            },
                            // 15s, confirmed - long enough to actually read the nametag and grab
                            // the buff, unlike the original 1s wither-essence animation this
                            // reuses.
                            EssenceEntityImpl { texture, lifetime_ticks: 300, blessing: Some(blessing_type) },
                        );
                    }
                }
            }

            Self::MimicChest => {
                let world = &mut player.server_mut().world;
                world.set_block_at(Blocks::Air, block_pos.x, block_pos.y, block_pos.z);
                world.interactable_blocks.remove(block_pos);

                // Spawn at the chest's own position (feet-level, matching where the block
                // was) - the AI's gravity/physics settles it onto the floor the same as any
                // other mob spawn. Needs the room it's in (same lookup the secret-count code
                // below already does, just moved up so both share one call) so `MobAiState`
                // knows which room's `entered` flag gates its own dormant/active state - not
                // that it matters in practice here, since interacting with the chest already
                // means the player is standing in this exact room, hence already entered.
                let spawn_pos = DVec3::new(block_pos.x as f64 + 0.5, block_pos.y as f64, block_pos.z as f64 + 0.5);
                if let Some(room_index) = player.server_mut().dungeon.get_room_at(block_pos.x, block_pos.z) {
                    let world = &mut player.server_mut().world;
                    let _ = crate::server::entity::dungeon_mobs::spawner::spawn_mimic(world, spawn_pos, 0.0, room_index);
                }

                // Finding the mimic counts as a secret too (real Hypixel behaviour), same
                // found_secrets/update_map_for_room mechanism as a normal `Self::Chest` open -
                // separate from the mimic_killed bonus score, which `combat::kill_mob` awards
                // when it's actually killed.
                if let Some(room_index) = player.server_mut().dungeon.get_room_at(block_pos.x, block_pos.z) {
                    let should_update_map = {
                        let room = player.server_mut().dungeon.rooms.get(room_index);
                        room.map(|r| r.found_secrets < r.room_data.secrets && r.entered).unwrap_or(false)
                    };

                    if let Some(room) = player.server_mut().dungeon.rooms.get_mut(room_index) {
                        if room.found_secrets < room.room_data.secrets {
                            room.found_secrets += 1;
                        }
                    }

                    if should_update_map {
                        player.server_mut().dungeon.update_map_for_room(room_index);
                    }
                }
            }

            Self::MushroomBottom { set_index: _ } => {
                // Debounce if already active
                if player.server_mut().world.tactical_insertions.iter().any(|(m, _)| m.client_id == player.client_id) {
                    return;
                }
                // Save precise origin and schedule return in 5s (100 ticks)
                let origin = player.position;
                let yaw = player.yaw;
                let pitch = player.pitch;

                // Teleport immediately to the corresponding UP point (center) by resolving within the player's current room
                let mut consumed_positions: Vec<BlockPos> = Vec::new();
                {
                    let dungeon = &mut player.server_mut().dungeon;
                    if let Some(room_index) = dungeon.get_player_room(player) {
                        let room_ref = &dungeon.rooms[room_index];
                        if let Some(set) = room_ref.mushroom_sets.iter().find(|s| s.bottom.iter().any(|bp| bp == block_pos)) {
                            if let Some(dest) = set.up.get(0) {
                                let pos = crate::server::utils::dvec3::DVec3::new(dest.x as f64 + 0.5, dest.y as f64, dest.z as f64 + 0.5);
                                player.server_teleport(pos, yaw, pitch, 0);
                                // Per explicit request: feels like stepping through a portal -
                                // max Nausea plus the same portal sound/particle combo teleport
                                // pads already use, cleared again the moment either return path
                                // (top click or the automatic timeout) fires - see
                                // `mushroom::apply_mushroom_up_effects`'s own doc comment.
                                crate::dungeon::room::mushroom::apply_mushroom_up_effects(player, pos);
                                // Per explicit correction: bottom and top are consumed
                                // independently, not together - clicking ANY bottom mushroom
                                // immediately retires every bottom mushroom in this set (so none
                                // of them can start a fresh trip again), but leaves the top
                                // mushroom(s) alone; a top click (below) does the mirror image.
                                consumed_positions.extend(set.bottom.iter().copied());
                            }
                        }
                    }
                }

                if !consumed_positions.is_empty() {
                    let world = player.world_mut();
                    for pos in consumed_positions {
                        world.interactable_blocks.remove(&pos);
                    }
                }

                let return_tick = player.world_mut().tick_count + 100;
                let marker = TacticalInsertionMarker {
                    client_id: player.client_id,
                    return_tick,
                    origin,
                    damage_echo_window_ticks: 0,
                    yaw,
                    pitch,
                    is_mushroom_secret: true,
                };
                // Optional cooldown sound pattern could be added to Vec
                player.world_mut().tactical_insertions.push((marker, Vec::<ScheduledSound>::new()));
            }

            Self::MushroomTop => {
                // Per explicit correction: consumed independently of the bottom mushroom(s) -
                // clicking ANY top mushroom immediately retires every top mushroom in this set,
                // leaving the bottom mushroom(s) alone (mirrors `MushroomBottom`'s own half above).
                let mut consumed_positions: Vec<BlockPos> = Vec::new();
                {
                    let dungeon = &mut player.server_mut().dungeon;
                    if let Some(room_index) = dungeon.get_player_room(player) {
                        let room_ref = &dungeon.rooms[room_index];
                        if let Some(set) = room_ref.mushroom_sets.iter().find(|s| s.top.iter().any(|bp| bp == block_pos)) {
                            consumed_positions.extend(set.top.iter().copied());
                        }
                    }
                }
                if !consumed_positions.is_empty() {
                    let world = player.world_mut();
                    for pos in consumed_positions {
                        world.interactable_blocks.remove(&pos);
                    }
                }

                // Only valid if active - specifically a mushroom trip's own marker, not some
                // unrelated active Tactical Insertion (the real item shares this same queue) a
                // player might also happen to have going.
                let world = player.world_mut();
                if let Some(idx) = world.tactical_insertions.iter().position(|(m, _)| m.client_id == player.client_id && m.is_mushroom_secret) {
                    let (marker, _) = world.tactical_insertions.remove(idx);
                    // Teleport immediately to origin and keep yaw/pitch
                    player.server_teleport(marker.origin, marker.yaw, marker.pitch, 0);
                    crate::dungeon::room::mushroom::clear_mushroom_up_effects(player, marker.origin);
                }
            }

            Self::WitherEssence { secret } => {
                let mut secret = secret.borrow_mut();
                
                // Only process if not already obtained
                if !secret.obtained {
                    let world = player.world_mut();
                    let server = player.server_mut();

                    // Send BlockAction for animation BEFORE removing the block
                    // Get the actual block and its state ID before removing the block
                    let block = world.chunk_grid.get_block_at(block_pos.x, block_pos.y, block_pos.z);
                    let block_state_id = block.get_block_state_id();
                    
                    // Send BlockAction to all players
                    // Try both event_id 0 and 1 - skull blocks might use different values
                    let block_center = (block_pos.x as f32 + 0.5, block_pos.y as f32 + 0.5, block_pos.z as f32 + 0.5);
                    for (_, other_player) in &mut server.world.players {
                        // Try event_id 0 first (some blocks use 0)
                        other_player.write_packet(&BlockAction {
                            block_pos: block_pos.clone(),
                            event_id: 0,
                            event_data: 1,
                            block_id: block_state_id,
                        });
                        // Also try event_id 1 (like chests)
                        other_player.write_packet(&BlockAction {
                            block_pos: block_pos.clone(),
                            event_id: 1,
                            event_data: 1,
                            block_id: block_state_id,
                        });
                    }

                    // Play note.harp sounds with ascending pitches (animation sequence)
                    // Schedule sounds to play at specific ticks with 0.25s (5 ticks) spacing
                    let pitches = [0.7936508, 0.8888889, 1.0, 1.0952381, 1.1904762];
                    let world_tick = world.tick_count;
                    let block_pos_f64 = (block_pos.x as f64, block_pos.y as f64, block_pos.z as f64);
                    
                    // Schedule fixed-position sounds
                    for (i, &pitch) in pitches.iter().enumerate() {
                        world.scheduled_fixed_sounds.push(crate::server::world::ScheduledFixedSound {
                            due_tick: world_tick + (i as u64 * 5), // 5 ticks apart (0.25s)
                            sound: Sounds::Harp,
                            volume: 1.0,
                            pitch,
                            pos_x: block_pos_f64.0,
                            pos_y: block_pos_f64.1,
                            pos_z: block_pos_f64.2,
                        });
                    }

                    world.set_block_at(Blocks::Air, block_pos.x, block_pos.y, block_pos.z);
                    world.interactable_blocks.remove(block_pos);

                    // Spawn essence entity with animation
                    use crate::dungeon::room::secrets::{EssenceEntityImpl, WITHER_ESSENCE_TEXTURE};
                    world.spawn_entity(
                        DVec3::from(block_pos).add_x(0.5).add_y(-1.4).add_z(0.5),
                        {
                            let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
                            metadata.is_invisible = true;
                            metadata
                        },
                        EssenceEntityImpl { texture: WITHER_ESSENCE_TEXTURE, lifetime_ticks: 20, blessing: None },
                    ).unwrap();

                    secret.obtained = true;

                    for other_player in server.world.players.values_mut() {
                        other_player.send_message("\u{a7}7You found a \u{a7}5Wither Essence\u{a7}7! Everyone gains an extra essence!");
                    }

                    // Increment found_secrets for the room containing this secret (only if not already obtained)
                    if let Some(room_index) = server.dungeon.get_room_at(block_pos.x, block_pos.z) {
                        let should_update_map = {
                            let room = server.dungeon.rooms.get(room_index);
                            room.map(|r| r.found_secrets < r.room_data.secrets && r.entered).unwrap_or(false)
                        };
                        
                        if let Some(room) = server.dungeon.rooms.get_mut(room_index) {
                            if room.found_secrets < room.room_data.secrets {
                                room.found_secrets += 1;
                            }
                        }
                        
                        // Update map if room is entered and secret count changed
                        if should_update_map {
                            server.dungeon.update_map_for_room(room_index);
                        }
                    }
                }
            }
            
            Self::Lever => {
                // Check if this lever unlocks any chests and unlock them
                {
                    let dungeon = &mut player.server_mut().dungeon;
                    if let Some(chest_positions) = dungeon.lever_to_chests.get(block_pos) {
                        // Unlock all chests linked to this lever
                        for chest_pos in chest_positions {
                            if let Some(chest_state) = dungeon.locked_chests.get_mut(chest_pos) {
                                if chest_state.locked {
                                    // Unlock the chest
                                    chest_state.locked = false;
                                }
                                // If already unlocked, do nothing (as per requirements)
                            }
                        }
                    }
                }
                
                let world = &mut player.server_mut().world;
                
                // Check if this is one of the special levers at the specified coordinates
                if is_special_lever(block_pos.x, block_pos.y, block_pos.z) {
                    // Toggle the lever power state
                    let current_power = world.redstone_system.get_power(*block_pos);
                    let new_power = if current_power > 0 { 0 } else { 15 };
                    world.redstone_system.set_power(*block_pos, new_power);
                    
                    // Get current lever block to toggle its state
                    let current_block = world.get_block_at(block_pos.x, block_pos.y, block_pos.z);
                    let new_lever_block = match current_block {
                        Blocks::Lever { orientation, powered } => {
                            // Toggle lever powered state
                            Blocks::Lever { 
                                orientation, 
                                powered: !powered 
                            }
                        }
                        _ => current_block, // Keep current block if not a lever
                    };
                    
                    // Update the lever block state
                    world.set_block_at(new_lever_block, block_pos.x, block_pos.y, block_pos.z);
                    
                    // Play vanilla lever click sound
                    let _ = player.write_packet(&SoundEffect {
                        sound: Sounds::RandomClick.id(),
                        volume: 0.3,
                        pitch: 0.8,
                        pos_x: block_pos.x as f64 + 0.5,
                        pos_y: block_pos.y as f64 + 0.5,
                        pos_z: block_pos.z as f64 + 0.5,
                    });
                    
                    // Send block change packet to update the lever visually
                    let _ = player.write_packet(&crate::net::protocol::play::clientbound::BlockChange {
                        block_pos: *block_pos,
                        block_state: new_lever_block.get_block_state_id(),
                    });
                    
                    // Send block action to show lever animation
                    let is_powered = match new_lever_block {
                        Blocks::Lever { powered, .. } => powered,
                        _ => false,
                    };
                    let _ = player.write_packet(&BlockAction {
                        block_pos: *block_pos,
                        event_id: 0, // Lever toggle
                        event_data: if is_powered { 1 } else { 0 }, // 1 for on, 0 for off
                        block_id: 69, // Lever block ID
                    });
                    
                    // Update redstone lanterns in the area
                    let is_powered = new_power > 0;
                    let new_lantern_block = if is_powered {
                        Blocks::LitRedstoneLamp
                    } else {
                        Blocks::RedstoneLamp
                    };
                    
                    // Check all positions around the lever for redstone lanterns
                    for x_offset in -1..=1 {
                        for y_offset in -1..=1 {
                            for z_offset in -1..=1 {
                                let check_pos = BlockPos {
                                    x: block_pos.x + x_offset,
                                    y: block_pos.y + y_offset,
                                    z: block_pos.z + z_offset,
                                };
                                
                                let current_block = world.get_block_at(check_pos.x, check_pos.y, check_pos.z);
                                if matches!(current_block, Blocks::RedstoneLamp | Blocks::LitRedstoneLamp) {
                                    // Update the lantern
                                    world.set_block_at(new_lantern_block, check_pos.x, check_pos.y, check_pos.z);
                                    
                                    // Send block change packet to update the lantern
                                    let _ = player.write_packet(&crate::net::protocol::play::clientbound::BlockChange {
                                        block_pos: check_pos,
                                        block_state: new_lantern_block.get_block_state_id(),
                                    });
                                }
                            }
                        }
                    }
                } else {
                    // Try to activate lever with falling blocks system
                    let dungeon = &mut player.server_mut().dungeon;
                    
                    // Find the room that contains this lever
                    let mut lever_found = false;
                    for room in &dungeon.rooms {
                        for lever_data in &room.lever_data {
                            let lever_pos = BlockPos {
                                x: lever_data.lever[0],
                                y: lever_data.lever[1],
                                z: lever_data.lever[2],
                            };
                            
                            if lever_pos == *block_pos {
                                // Play lever click sound
                                let _ = player.write_packet(&SoundEffect {
                                    sound: Sounds::RandomClick.id(),
                                    volume: 0.3,
                                    pitch: 0.8,
                                    pos_x: block_pos.x as f64 + 0.5,
                                    pos_y: block_pos.y as f64 + 0.5,
                                    pos_z: block_pos.z as f64 + 0.5,
                                });
                                
                                // Play anvil break sound immediately
                                let _ = player.write_packet(&SoundEffect {
                                    sound: "random.anvil_break",
                                    volume: 1.0,
                                    pitch: 1.7,
                                    pos_x: player.position.x,
                                    pos_y: player.position.y,
                                    pos_z: player.position.z,
                                });
                                
                                // Send opening message
                                player.send_message("§cYou hear the sound of something opening...");
                                
                                // Implement falling blocks animation (similar to wither doors)
                                for block_pos in &lever_data.blocks {
                                    let block_world_pos = BlockPos {
                                        x: block_pos[0],
                                        y: block_pos[1],
                                        z: block_pos[2],
                                    };
                                    
                                    // Get the current block type before replacing with barrier
                                    let current_block = world.get_block_at(block_world_pos.x, block_world_pos.y, block_world_pos.z);
                                    
                                    // Skip air blocks
                                    if matches!(current_block, Blocks::Air) {
                                        continue;
                                    }
                                    
                                    // Play RandomWoodClick sound 8 times with 250ms intervals from this block position
                                    for i in 0..8 {
                                        let delay_ticks = i * 5; // 250ms = 5 ticks at 20 TPS
                                        world.server_mut().schedule(delay_ticks, move |server| {
                                            for (_, player) in &mut server.world.players {
                                                let _ = player.write_packet(&crate::net::protocol::play::clientbound::SoundEffect {
                                                    sound: crate::server::utils::sounds::Sounds::RandomWoodClick.id(),
                                                    pos_x: block_world_pos.x as f64 + 0.5,
                                                    pos_y: block_world_pos.y as f64 + 0.5,
                                                    pos_z: block_world_pos.z as f64 + 0.5,
                                                    volume: 2.0,
                                                    pitch: 0.49,
                                                });
                                            }
                                        });
                                    }
                                    
                                    // Set barrier block to prevent passage
                                    world.set_block_at(Blocks::Barrier, block_world_pos.x, block_world_pos.y, block_world_pos.z);
                                    
                                    // Schedule the barrier block to be replaced with air after 20 ticks
                                    world.server_mut().schedule(20, move |server| {
                                        server.world.set_block_at(Blocks::Air, block_world_pos.x, block_world_pos.y, block_world_pos.z);
                                    });
                                    
                                    // Spawn falling block entity
                                    let _ = world.spawn_entity(
                                        crate::server::utils::dvec3::DVec3::new(
                                            block_world_pos.x as f64 + 0.5, 
                                            block_world_pos.y as f64 - 0.65, 
                                            block_world_pos.z as f64 + 0.5
                                        ),
                                        {
                                            let mut metadata = crate::server::entity::entity_metadata::EntityMetadata::new(crate::server::entity::entity_metadata::EntityVariant::Bat { hanging: false });
                                            metadata.is_invisible = true;
                                            metadata
                                        },
                                        crate::dungeon::room::levers::LeverEntityImpl::new(current_block, 5.0, 20),
                                    );
                                }
                                
                                // Schedule removal of barrier blocks after 20 ticks
                                let blocks_to_remove = lever_data.blocks.clone();
                                world.server_mut().schedule(20, move |server| {
                                    for block_pos in blocks_to_remove {
                                        let block_world_pos = BlockPos {
                                            x: block_pos[0],
                                            y: block_pos[1],
                                            z: block_pos[2],
                                        };
                                        server.world.set_block_at(Blocks::Air, block_world_pos.x, block_world_pos.y, block_world_pos.z);
                                    }
                                });
                                
                                // Remove the lever from interactable blocks so it can only be used once
                                world.interactable_blocks.remove(block_pos);
                                
                                lever_found = true;
                                break;
                            }
                        }
                        if lever_found {
                            break;
                        }
                    }
                    
                    if !lever_found {
                        // Lever not found in any room - play click sound and used message
                        let _ = player.write_packet(&SoundEffect {
                            sound: "random.click",
                            volume: 0.3,
                            pitch: 0.49,
                            pos_x: block_pos.x as f64,
                            pos_y: block_pos.y as f64,
                            pos_z: block_pos.z as f64,
                        });
                        
                        // Send already used message
                        player.send_message("§cThis lever has already been used.");
                    }
                }
            }
            
            Self::RedstoneKeySkull { .. } => {
                // Prevent picking up redstone key if player already has one
                if player.has_redstone_key {
                    return;
                }
                
                // Remove the skull block
                let world = player.world_mut();
                world.set_block_at(Blocks::Air, block_pos.x, block_pos.y, block_pos.z);
                world.interactable_blocks.remove(block_pos);
                
                // Play sound: random.pop, volume 1, pitch 1
                player.write_packet(&SoundEffect {
                    sound: Sounds::Pop.id(),
                    volume: 1.0,
                    pitch: 1.0,
                    pos_x: block_pos.x as f64 + 0.5,
                    pos_y: block_pos.y as f64 + 0.5,
                    pos_z: block_pos.z as f64 + 0.5,
                });
                
                // Store RedstoneKey on player (not in inventory)
                player.has_redstone_key = true;
                
                // Send message: &r&aYou found a Secret Redstone Key!&r
                use crate::server::player::dungeon_stats::legacy_to_chat_component;
                use crate::net::protocol::play::clientbound::Chat;
                player.write_packet(&Chat {
                    component: legacy_to_chat_component("&r&aYou found a Secret Redstone Key!&r"),
                    chat_type: 0,
                });
            }
            
            Self::ThreeWeirdosChest { chest_index, state } => {
                crate::dungeon::room::three_weirdos::interact_chest(player, block_pos, *chest_index, state);
            }

            Self::CreeperBeamsLantern { state } => {
                crate::dungeon::room::creeper_beams::interact_lantern(player, block_pos, state);
            }

            Self::TicTacToeButton { state, index } => {
                crate::dungeon::room::tic_tac_toe::interact_cell(player, *index, state);
            }

            Self::QuizButton { state, answer_index } => {
                crate::dungeon::room::quiz::interact_button(player, block_pos, *answer_index, state);
            }

            Self::BoulderTrigger { state, grid_x, grid_z, direction } => {
                crate::dungeon::room::boulder::handle_push(player, state, *grid_x, *grid_z, *direction);
            }

            Self::WaterboardLever { state, lever } => {
                crate::dungeon::room::waterboard::handle_lever(player, state, *lever);
            }

            Self::Callback(func) => {
                func(player, block_pos);
            }
        }
    }
}
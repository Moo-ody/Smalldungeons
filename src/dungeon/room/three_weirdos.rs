//! Three Weirdos puzzle: 3 named NPCs, each standing next to a chest, make one statement each
//! (from 6 possible sets of 3) about which chest holds the reward - exactly one statement per
//! set is always true, and whichever physical NPC ends up saying it (after per-round name/role
//! shuffling) is standing next to the correct chest.
//!
//! Data (positions, name pool, all 6 dialogue templates, the 3 NPC skins) and the shuffle/
//! substitution logic are ported from RustClear-main's reference implementation
//! (`dungeon/room/puzzles/three_weirdos.rs`), adapted from its bevy-ECS `Component`/
//! `RoomImplementation` architecture to this codebase's plain `EntityImpl` +
//! `world.interactable_blocks` pattern (the same shape `locked_chests.rs`/`levers.rs` already
//! use). Session research cross-checked RustClear's "always-correct" statement per template
//! against the Skyblocker mod's actual solver regex/tests (open source, battle-tested against
//! the live game) - all 6 matched exactly, confirming this data/logic is correct, not guessed.

use crate::dungeon::room::room::Room;
use crate::net::protocol::play::clientbound::{BlockAction, Particles, PlayerListItem, SoundEffect, Teams};
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::net::var_int::VarInt;
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::block::rotatable::Rotatable;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::{ClientId, GameProfile, GameProfileProperty, Player};
use crate::server::player::scoreboard::CREATE_TEAM;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::particles::ParticleTypes;
use crate::server::utils::player_list::player_profile::{GameType, PlayerData};
use crate::server::utils::sized_string::SizedString;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;
use crate::utils::seeded_rng::seeded_rng;
use rand::prelude::{IndexedRandom, SliceRandom};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use uuid::Uuid;

/// Room-relative (pre-rotation) chest positions - `CHEST_POSITIONS[i]` is `WEIRDO_POSITIONS[i]`'s
/// own chest.
const CHEST_POSITIONS: [BlockPos; 3] = [
    BlockPos { x: 18, y: 69, z: 24 },
    BlockPos { x: 16, y: 69, z: 25 },
    BlockPos { x: 14, y: 69, z: 24 },
];

const WEIRDO_POSITIONS: [BlockPos; 3] = [
    BlockPos { x: 17, y: 69, z: 24 },
    BlockPos { x: 15, y: 69, z: 25 },
    BlockPos { x: 13, y: 69, z: 24 },
];

const WEIRDO_NAMES: [&str; 21] = [
    "§cArdis", "§cBaxter", "§cBenson", "§cCarver", "§cElmo", "§cEveleth", "§cHope", "§cHugo",
    "§cLino", "§cLuverne", "§cMadelia", "§cMarshall", "§cMelrose", "§cMontgomery", "§cMorris",
    "§cRamsey", "§cRose", "§cVictoria", "§cVirginia", "§cWillmar", "§cWinona",
];

/// 6 possible statement sets, each `[role "1", role "2", role "3"]` - `"1"`/`"2"`/`"3"` inside
/// the text get substituted with the round's 3 chosen names at generation time. Slot 0 in every
/// set is deliberately always the objectively-true statement (verified against Skyblocker's
/// solver - see the module doc comment): whichever physical NPC ends up speaking it (after
/// shuffling) is standing next to the correct chest.
const DIALOGUE: [[&str; 3]; 6] = [
    [
        "§e[NPC] §c1§r: The reward is not in my chest!",
        "§e[NPC] §c2§r: One of us is telling the truth!",
        "§e[NPC] §c3§r: They are both telling the truth. The reward isn't in §c1's§r chest.",
    ],
    [
        "§e[NPC] §c1§r: The reward isn't in any of our chests.",
        "§e[NPC] §c2§r: The reward is not in my chest. They are both lying.",
        "§e[NPC] §c3§r: The reward is in my chest!",
    ],
    [
        "§e[NPC] §c1§r: My chest doesn't have the reward. We are all telling the truth.",
        "§e[NPC] §c2§r: My chest doesn't have the reward. At least one of the others is telling the truth!",
        "§e[NPC] §c3§r: One of the others is lying!",
    ],
    [
        "§e[NPC] §c1§r: My chest has the reward and I'm telling the truth!",
        "§e[NPC] §c2§r: They are both lying, the reward is in my chest!",
        "§e[NPC] §c3§r: They are both telling the truth, the reward is in §c2's§r chest!",
    ],
    [
        "§e[NPC] §c1§r: At least one of them is lying, and the reward is not in §c3's§r chest!",
        "§e[NPC] §c2§r: We are all telling the truth!",
        "§e[NPC] §c3§r: §c2§r is telling the truth and the reward is in his chest.",
    ],
    [
        "§e[NPC] §c1§r: Both of them are telling the truth. Also, §c2§r has the reward in their chest!",
        "§e[NPC] §c2§r: §c3§r is telling the truth.",
        "§e[NPC] §c3§r: My chest has the reward!",
    ],
];

/// Real signed skins for the 3 NPC slots (index-paired with whichever name/position ends up
/// there after shuffling - the skin follows the physical slot, not the name).
const SKINS: [(&str, &str); 3] = [
    (
        "eyJ0aW1lc3RhbXAiOjE1ODIxNDYwNjAxMDYsInByb2ZpbGVJZCI6ImEyZjgzNDU5NWM4OTRhMjdhZGQzMDQ5NzE2Y2E5MTBjIiwicHJvZmlsZU5hbWUiOiJiUHVuY2giLCJzaWduYXR1cmVSZXF1aXJlZCI6dHJ1ZSwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwOi8vdGV4dHVyZXMubWluZWNyYWZ0Lm5ldC90ZXh0dXJlLzdiNGM2ZjVkZjMxMzRhZGY0YTdlYWUxMmZlMjJlYjZhYTEwMmI2NzM1MjIxZTdmNTQ3NWM3YmJlYzQyMzdiYjgifX19",
        "rDW4GM5nUP2hvfh9it3pfXgGeaDoa+JEHoOefy5Rwruz2clabGqda1lXt527QWTAWieS4lFcNWnqwJUtzLow83i/kbFZ72MkUTo3c0LC3nFDTtABGijY8KfcIVRp0XHzWdQwG7PXWYt5RvX+RgEdOmd+yhDoq16Cf4d3MhWhuFrSpKJohzvQ3ad/FFXdpSiWmklnsQ2n7ZP1ZRzuWWg4kRdtYEEjE2oodVkQoN8xqtddK+eT/3kz9n/aqPfokAHjWMJDbkqPBLweLVK2+WYkI9c6unHcG/uWKwhw8lwG7oEXLNhtDnipoWqA+TNcP//m8DAF9kA2MeBjO72U2v+UkNIGXZPamy5wSqhoNhyTAmG0MsammQprwfzL/K3PVW5QZxIldAIDMFNn/T6tYH2PtT345A+0gC0xtZUXHjscjlok/dcvYyleHyxK15fPyYtxcmGE59AUjj0Xllv90aEECRHrzC3t+2/gj+nWcDrLPvxX/qbjlTXKyxT/V0vJlMrzfoj8apPHgdj3S3mu3XDog18kfj7iPmoN0X1xllGzgR4SmOqnlCSWKFieYx7wrbN9J1y23itVteto9DiMWKbgc314m6nxGSaiSVSZriMX4lciNv1js7ADkyQ5LX3FuWUe7KRJuHYv/aRSzj70IEHq+/G6I5EHd3WbRJKmM6AMg4Q=",
    ),
    (
        "eyJ0aW1lc3RhbXAiOjE1ODIxNDU3Njc3MDEsInByb2ZpbGVJZCI6ImEyZjgzNDU5NWM4OTRhMjdhZGQzMDQ5NzE2Y2E5MTBjIiwicHJvZmlsZU5hbWUiOiJiUHVuY2giLCJzaWduYXR1cmVSZXF1aXJlZCI6dHJ1ZSwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwOi8vdGV4dHVyZXMubWluZWNyYWZ0Lm5ldC90ZXh0dXJlLzE0Nzk0Yjc3N2Y5YmIzNDhjNTBiMTlhNTAwZDk2ODkwNGM1NzAwNTRlZGMxNzhhNTIwYzRlMWEyZGEwZDY1ODcifX19",
        "k+o7OResg2zOWnG/s2kdEWgEytoj4sVcaFHU0oiLkhwZfxp9NpEeGdTC3A8PdiGiLGAMGJnmBjgXPY0iyJJbfpe+f8FLb/F0FpFPT8P6t7Et9t/jep8ZtAKRQF8kKkgTvDljycSSQQod/DXHOuX/9LCb0UytSc8FFeaTKStTqbGpAOA9Wgb3cF9Mg5DmKC3RJwOeMw3G1nnWrJuG8W+6Uf7oR8f4+M6DBxay3GDNRNBVRQfFxovd/3T0f9VBCenn5ednB+T4t5h8pEssUVmgGiHLwJONxfQolzQkgb7sVC8I9oi74JDwAg3k4rKFb64YTbVKsvbmqb+sqfrcbZUQhPLR9BgNKlWr/A3SokplfjraK7+m90B/vM4jZiFnexxpW1TwNGZkrDrkozNckYRtTK2j9PmOwUgscrRMl4pMNMe7bPDpb1w8PeAzXMSTYzcvVQDK4rFuGCviwWq5JhQnbI+sFTHCJtxixH1AQSWTnLIqUqcKNtQsVYgoN3AGpaj3v5cqoGHh2WPjo1vOMNQN1VjCpNGMiNEJhf9xxfigh4bdC4thH4NMBKavXeMedJ5M9azmBo30b9u0YT3nYbqrx82D4HxagmKeb2+j2O3StdGWk1VUUfxUpwQ4mR9nMKfN1k2JYewog3uxGcJ9FGAcOflvQqBwSIVrMnZgtHFpuQ0=",
    ),
    (
        "eyJ0aW1lc3RhbXAiOjE1ODIwNTY1MjY1ODksInByb2ZpbGVJZCI6ImEyZjgzNDU5NWM4OTRhMjdhZGQzMDQ5NzE2Y2E5MTBjIiwicHJvZmlsZU5hbWUiOiJiUHVuY2giLCJzaWduYXR1cmVSZXF1aXJlZCI6dHJ1ZSwidGV4dHVyZXMiOnsiU0tJTiI6eyJ1cmwiOiJodHRwOi8vdGV4dHVyZXMubWluZWNyYWZ0Lm5ldC90ZXh0dXJlLzYxOTBhN2IyYmIxM2FlNTgyM2Y2YTE4NDZmODQyYzM0ODllMjYyMGEzNjY1NTc0YTBmYmE5NzVjMzk0MTA5MjIifX19",
        "D/AAFPkwp3dqEx8OktDtqX0PSwJfu6PS+u67e8mq+0FMz+yqvDhD4FmzlvJlz6dwVa+UWGBCX6CMPXbPja9eeR90GFEYU+AYInam8IvyrmDzw7q0Fx3jzP9aRmHSn4229Y8GXhkOJ37k3pWf5zrcIJmT9npIq4lwEc3B0OxEZtQadanWX0/qIr/bpbrB+en2zIWzzwQWIAXPJUwQgiVj7mRwfMCyajoOqGs0AApzTi5IPresYF2BZZ9pLWyLv96YFhm96ncMHVJlSl3h8mt0R1pGi2BwOROYIfFq6HDONpSfD3R7aaty0fyPeV9kcrswndCS5/ubZxvv1bLp2wqhR0A5NzWr2GM3GK7o2EQgM5o9gsKS65SGPaWF0h3dUzsrpCOSMKzxzj29eAP4TLsLxWNAyaR/Q4NQt8cluiisLSk2yKUDovzUiSqrdLToD+5DPFqNYxDRraCc2gQQlsHpp3aXHNpqoBYcczTXwUHgjqh71HodzXGo4pxOcJNo5kOjV+uyfgvR9zCIuoN3j7UK6E2F3LQrLDTTRb8W4KGkMDnEESv8jGrXwQs4WzqUdP4HHXOFHbp2cx0Pi7xc93MQW1ZmjNtDeLr+xvsqDbL2syI8D2mf2++vjLhPSzxqtx4hOcyWQFCMC9Sdzb/bDg8JAuXgQSAMgmp1E1Oz/msZJ1w=",
    ),
];

/// Farewell lines an NPC gives once the puzzle has already been resolved (either outcome) -
/// "name" is replaced with that specific NPC's own chosen name.
const FINISHED_DIALOGUE: [&str; 4] = [
    "§e[NPC] §cname§f: You're free to leave.§7",
    "§e[NPC] §cname§f: You can leave now! Bye!§7",
    "§e[NPC] §cname§f: Thanks for playing! Now get out!§7",
    "§e[NPC] §cname§f: Scram!§7",
];

/// Shared per-room puzzle state - one instance jointly owned (`Rc<RefCell<_>>`) by the 3 NPC
/// entities' interact handlers and the 3 chests' `BlockInteractAction::ThreeWeirdosChest`
/// entries, same idiom `BlockInteractAction::Chest`'s `secret` field already uses for its
/// shared `DungeonSecret`.
#[derive(Debug)]
pub struct ThreeWeirdosState {
    room_index: usize,
    names: [&'static str; 3],
    dialogues: [String; 3],
    correct_index: usize,
    /// World positions of the 3 chests, index-paired with `names`/`dialogues` - precomputed
    /// once at setup so `WeirdoImpl::interact`'s correct-chest flame effect doesn't need to
    /// re-derive them (room rotation + corner offset) on every click.
    chest_world_positions: [BlockPos; 3],
    /// Per-player: which of the 3 NPCs (by physical slot) they've talked to.
    clicked_weirdos: HashMap<ClientId, [bool; 3]>,
    /// True once any chest has been opened - solved or failed, either way the puzzle is done
    /// and further chest clicks/NPC talk just give flavor responses.
    resolved: bool,
}

/// Sets up the Three Weirdos puzzle for `room` if it actually is one - spawns the 3 NPCs and
/// registers the 3 chests as interactable. No-op for every other room (belt-and-suspenders check;
/// the caller already filters by name and by `Room::weirdos_spawned` before calling this at all).
/// Called once from `Dungeon::tick`'s room-entry hook, the first time a player actually crosses
/// into the room - deliberately NOT at room load (`Room::load_into_world`) like most other
/// puzzle setups, so the NPCs don't exist yet for a player peeking in from an adjacent room or a
/// chunk simply being loaded. See `Room::weirdos_spawned`'s doc comment for why this can't
/// double-fire.
pub fn setup(room: &Room, room_index: usize, world: &mut World) {
    if room.room_data.name != "Three Weirdos" {
        return;
    }

    let mut rng = seeded_rng();

    let chosen_names: [&str; 3] = WEIRDO_NAMES
        .choose_multiple(&mut rng, 3)
        .cloned()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let chosen_dialogue = *DIALOGUE.choose(&mut rng).expect("DIALOGUE is non-empty");

    let mut indices: [usize; 3] = [0, 1, 2];
    indices.shuffle(&mut rng);

    let mut names: [&'static str; 3] = [""; 3];
    let mut dialogues: [String; 3] = [const { String::new() }; 3];
    let mut correct_index = 0;

    for (slot, &role) in indices.iter().enumerate() {
        names[slot] = chosen_names[role];
        dialogues[slot] = chosen_dialogue[role]
            .replace('1', chosen_names[0])
            .replace('2', chosen_names[1])
            .replace('3', chosen_names[2]);
        if role == 0 {
            correct_index = slot;
        }
    }

    let chest_world_positions: [BlockPos; 3] = std::array::from_fn(|i| room.get_world_block_pos(&CHEST_POSITIONS[i]));

    let state = Rc::new(RefCell::new(ThreeWeirdosState {
        room_index,
        names,
        dialogues,
        correct_index,
        chest_world_positions,
        clicked_weirdos: HashMap::new(),
        resolved: false,
    }));

    for (index, relative) in WEIRDO_POSITIONS.into_iter().enumerate() {
        let world_pos = room.get_world_block_pos(&relative);
        let position = DVec3::new(world_pos.x as f64 + 0.5, world_pos.y as f64, world_pos.z as f64 + 0.5);
        let yaw = 180.0_f32.rotate(room.rotation);

        let (texture_value, texture_signature) = SKINS[index];
        let mut metadata = EntityMetadata::new(EntityVariant::Player);
        metadata.ai_disabled = true;

        let uuid = Uuid::new_v4();
        let username: String = names[index].chars().take(16).collect();

        // Broadcast the tab-list skin entry immediately (not just via `WeirdoImpl::spawn`'s own
        // queued write below) - same reasoning as `spawner.rs`'s `spawn_active_mob`: a
        // `SpawnPlayer` packet resolves its skin purely from the client's tab-list cache, and
        // `spawn_entity_with_uuid` writes `SpawnPlayer` into the chunk buffer before calling
        // `entity_impl.spawn()`, so relying on that alone would (for anyone already viewing this
        // chunk) show a blank/default skin instead.
        crate::server::entity::dungeon_mobs::spawner::register_npc_tab_list_entry(world, uuid, &username, texture_value, Some(texture_signature));

        let entity_impl = WeirdoImpl {
            index,
            state: state.clone(),
            uuid,
            username,
            texture_value: texture_value.to_string(),
            texture_signature: texture_signature.to_string(),
        };

        if let Ok(entity_id) = world.spawn_entity_with_uuid(position, metadata, entity_impl, Some(uuid)) {
            if let Some((entity, _)) = world.entities.get_mut(&entity_id) {
                entity.yaw = yaw;
            }

            // A real (unstylable, uncolored) vanilla player nameplate isn't used for these NPCs
            // (see the hidden-nameplate team in `WeirdoImpl::spawn`) - this invisible, named
            // armor stand is the actual visible floating name instead, same convention as Mort's
            // and every dungeon mob's own nametag (`spawn_following_nametag`). It also happens to
            // be exactly what OdinClient's real `WeirdosSolver` looks for: it finds the correct
            // chest by searching for an `EntityArmorStand` whose name matches the NPC named in
            // chat, not the player-model entity itself - confirmed against Odin's actual source
            // (`WeirdosSolver.kt`'s `onNPCMessage`) - so without this armor stand, Odin's solver
            // would never find anything to highlight even though the chat/logic is correct.
            let _ = crate::server::entity::spawn_equipped::spawn_following_nametag(world, entity_id, names[index], 1.1, EntityVariant::ArmorStand);

            // Second line directly beneath the name, same "stack two following-nametag armor
            // stands 0.25 apart" convention `main.rs` already uses for Mort's own name+CLICK
            // pair. `spawn_following_nametag` registers this under `entity_id` in
            // `world.entity_following_nametag` alongside the name label above (a `Vec`, not a
            // single slot - already proven to hold more than one child per host by Mort's own
            // pair), so `world.despawn_entity(entity_id)` tears down both automatically; nothing
            // extra to track here. It's a separate, invisible, hitbox-less-for-interaction-
            // purposes armor stand (no `EntityImpl::interact` override, unlike `WeirdoImpl`
            // below), positioned by `FollowingNametagImpl` every tick from `entity_id`'s own
            // position - if the weirdo ever moved, this would move with it for free, though today
            // it doesn't (see `tick` below - rotation only).
            let _ = crate::server::entity::spawn_equipped::spawn_following_nametag(world, entity_id, "§e§lCLICK", 0.85, EntityVariant::ArmorStand);
        }
    }

    for (chest_index, relative) in CHEST_POSITIONS.into_iter().enumerate() {
        let world_pos = room.get_world_block_pos(&relative);
        world.interactable_blocks.insert(world_pos, BlockInteractAction::ThreeWeirdosChest {
            chest_index,
            state: state.clone(),
        });
    }
}

/// `BlockInteractAction::ThreeWeirdosChest`'s handler - a player opened chest `chest_index`.
pub fn interact_chest(player: &mut Player, block_pos: &BlockPos, chest_index: usize, state: &Rc<RefCell<ThreeWeirdosState>>) {
    let mut data = state.borrow_mut();
    if data.resolved {
        return;
    }
    let mut failed = false;

    let talked_to_all = data.clicked_weirdos.get(&player.client_id).is_some_and(|clicked| clicked.iter().all(|c| *c));
    if !talked_to_all {
        player.send_message("§ctalk to us");
        return;
    }

    data.resolved = true;
    let room_index = data.room_index;

    if chest_index == data.correct_index {
        let message = format!(
            "§a§lPUZZLE SOLVED! §a{} §ewasn't fooled by §c{}§e! §4G§co§6o§ed §2j§bo§3b§5!",
            player.profile.username, data.names[chest_index],
        );
        let block_action = BlockAction {
            block_pos: *block_pos,
            event_id: 1,
            event_data: 1,
            block_id: 54,
        };
        for other in player.server_mut().world.players.values_mut() {
            other.send_message(&message);
            other.write_packet(&block_action);
        }
    } else {
        player.send_message(&format!("§e[NPC] §c{}§f: You fool!", data.names[chest_index]));
        let message = format!(
            "§c§lPUZZLE FAIL! §a{} §ewas fooled by §c{}§e! §4Y§ci§6k§ee§as§2!",
            player.profile.username, data.names[chest_index],
        );
        // Also broadcasts a `BlockAction` (chest-open) for the wrong chest, same as the solved
        // branch above - not just for the visual (the chest still briefly "opens" before it's
        // cleared below), but because OdinClient's own real puzzle-timer/personal-best tracking
        // (`PuzzleSolvers.kt`'s `S24PacketBlockAction` handler) treats *any* `BlockAction` at one
        // of the 3 known chest positions as "the puzzle is over," win or lose - without this, a
        // failed attempt would never stop Odin's timer.
        let block_action = BlockAction {
            block_pos: *block_pos,
            event_id: 1,
            event_data: 1,
            block_id: 54,
        };
        for other in player.server_mut().world.players.values_mut() {
            other.send_message(&message);
            other.write_packet(&block_action);
        }
        player.write_packet(&SoundEffect {
            sound: Sounds::RandomExplode.id(),
            pos_x: block_pos.x as f64,
            pos_y: block_pos.y as f64,
            pos_z: block_pos.z as f64,
            volume: 1.0,
            pitch: 1.0,
        });

        // All 3 chests get cleared on a fail, not just the wrong one - matches the real puzzle's
        // fail visual (RustClear's reference implementation confirmed this).
        let world = player.world_mut();
        for pos in data.chest_world_positions {
            world.set_block_at(Blocks::Air, pos.x, pos.y, pos.z);
            world.interactable_blocks.remove(&pos);
        }

        player.server_mut().dungeon.record_puzzle_failed();
        failed = true;
    }

    if let Some(room) = player.server_mut().dungeon.rooms.get_mut(room_index) {
        room.puzzle_completed = true;
        room.puzzle_failed = failed;
    }
    player.server_mut().dungeon.update_map_for_room(room_index);
}

struct WeirdoImpl {
    index: usize,
    state: Rc<RefCell<ThreeWeirdosState>>,
    uuid: Uuid,
    username: String,
    texture_value: String,
    texture_signature: String,
}

impl EntityImpl for WeirdoImpl {
    fn spawn(&mut self, entity: &mut Entity, buffer: &mut crate::net::packets::packet_buffer::PacketBuffer) {
        let mut properties = HashMap::new();
        properties.insert("textures".to_string(), GameProfileProperty {
            value: self.texture_value.clone(),
            signature: Some(self.texture_signature.clone()),
        });

        let player_data = PlayerData {
            ping: 0,
            game_mode: GameType::Survival,
            profile: GameProfile {
                uuid: self.uuid,
                username: self.username.clone(),
                properties,
            },
            display_name: Some(ChatComponentTextBuilder::new(self.username.clone()).build()),
        };

        buffer.write_packet(&PlayerListItem {
            action: VarInt(0), // ADD_PLAYER
            players: vec![&player_data],
        });

        // Hides the vanilla nameplate (bound to the profile username, unstylable) - the weirdo's
        // real styled name comes through its dialogue lines instead, matching how
        // `DungeonPlayerMobImpl` hides its own NPC nameplates the same way. Keyed by the
        // entity's own unique id, not `self.index` (0/1/2 repeats per room) - a shared literal
        // team name across multiple entities has already caused a real "team already exists"
        // bug elsewhere in this codebase once before (see `spawner.rs`'s own note on this).
        let team_name = format!("weird{}", entity.id);
        buffer.write_packet(&Teams {
            name: SizedString::truncated(&team_name),
            display_name: SizedString::truncated(""),
            prefix: SizedString::truncated(""),
            suffix: SizedString::truncated(""),
            name_tag_visibility: SizedString::truncated("never"),
            color: 0,
            players: vec![SizedString::truncated(&self.username)],
            action: CREATE_TEAM,
            friendly_flags: 0,
        });
    }

    fn tick(&mut self, entity: &mut Entity, _buffer: &mut crate::net::packets::packet_buffer::PacketBuffer) {
        // Same "look at the nearest player" reaction Mort gets (see `main.rs`'s `MortImpl::tick`)
        // - a weirdo isn't on the dungeon-mob AI pipeline at all (no combat, no wandering), so
        // this is its only motion, matching the request that they track the player like Mort does.
        const FACE_PLAYER_RANGE: f64 = 8.0;
        let npc_pos = entity.position;
        let world = entity.world_mut();
        let nearest_player_pos = world.players.values()
            .map(|player| (player.position, player.position.distance_to(&npc_pos)))
            .filter(|(_, dist)| *dist <= FACE_PLAYER_RANGE)
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(pos, _)| pos);

        if let Some(player_pos) = nearest_player_pos {
            crate::server::entity::dungeon_mobs::ai::movement::face_toward(entity, player_pos);
        }
    }

    fn interact(&mut self, entity: &mut Entity, player: &mut Player, action: &EntityInteractionType) -> bool {
        // Punching a weirdo talks to it too, same as right-clicking - real vanilla NPCs on
        // Hypixel generally respond to either, and there's no separate "attack" behavior here
        // (these NPCs can't be killed - see `spawn`'s lack of `CombatState` registration).
        if !matches!(action, EntityInteractionType::Interact | EntityInteractionType::Attack) {
            return false;
        }

        let mut data = self.state.borrow_mut();
        let clicked = data.clicked_weirdos.entry(player.client_id).or_insert([false; 3]);
        clicked[self.index] = true;

        if !data.resolved {
            let message = data.dialogues[self.index].clone();
            player.send_message(&message);
        } else {
            let chosen = FINISHED_DIALOGUE.choose(&mut rand::rng()).expect("FINISHED_DIALOGUE is non-empty");
            player.send_message(&chosen.replace("name", data.names[self.index]));
        }

        player.write_packet(&SoundEffect {
            sound: Sounds::DonkeyHit.id(),
            pos_x: entity.position.x,
            pos_y: entity.position.y,
            pos_z: entity.position.z,
            volume: 1.0,
            pitch: 0.5,
        });

        // A subtle tell for anyone who knows to look for it: clicking the weirdo actually
        // holding the reward puffs a flame in front of his own chest, independent of whether the
        // puzzle has been resolved yet.
        if self.index == data.correct_index {
            let chest_pos = data.chest_world_positions[self.index];
            // `+0.5` on Y alone lands dead center of the chest block's own bounding cube - inside
            // its solid model, where the opaque chest mesh can hide particles rendered "behind"
            // it from the camera's point of view. `+1.0` clears the lid (chests are well under a
            // full block tall) so the flame floats visibly in open air just above it instead.
            let particles = Particles {
                particle_id: ParticleTypes::Flame.get_id(),
                long_distance: true,
                x: chest_pos.x as f32 + 0.5,
                y: chest_pos.y as f32 + 1.0,
                z: chest_pos.z as f32 + 0.5,
                offset_x: 0.0,
                offset_y: 0.0,
                offset_z: 0.0,
                speed: 0.0,
                count: 1,
            };
            for other in entity.world_mut().players.values_mut() {
                other.write_packet(&particles);
            }
        }

        true
    }
}

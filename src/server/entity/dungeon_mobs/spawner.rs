//! Spawns dungeon mobs for a room from its recorded mob data (see `spawn_data.rs`).
//!
//! This is the only place that turns JSON spawn/placement entries into actual entities -
//! everything else in `dungeon_mobs` just describes archetypes or converts data. No mob gets
//! a bespoke struct here: every entry is spawned through the same handful of code paths,
//! parameterized by its [`DungeonMobType`]/[`MobBaseKind`] and JSON equipment/position.

use crate::dungeon::room::room::Room;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{PacketEntityMetadata, PlayerListItem, Teams};
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::net::var_int::VarInt;
use crate::server::block::block_position::BlockPos;
use crate::server::block::rotatable::Rotatable;
use crate::server::entity::dungeon_mobs::ai::attack::AttackModule;
use crate::server::entity::dungeon_mobs::ai::profile::profile_for;
use crate::server::entity::dungeon_mobs::ai::state::MobAiState;
use crate::server::entity::dungeon_mobs::ai::{run_mob_ai, DungeonMobAiImpl};
use crate::server::entity::dungeon_mobs::equipment_convert::convert_equipment;
use crate::server::entity::dungeon_mobs::mob_type::{format_health, lost_adventurer_skin_for_helmet, DungeonMobType, MobBaseKind};
use crate::server::entity::dungeon_mobs::spawn_data::{get_room_mob_spawns, MobSpawnJson};
use crate::server::entity::entity::{Entity, EntityId, EntityImpl, NoEntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::entity::equipment::Equipment;
use crate::server::entity::spawn_equipped::{send_equipment_packets, spawn_following_nametag, AISuspended, AttackCooldown, CombatState};
use crate::server::items::item_stack::{ItemStack, ItemStackExt};
use crate::server::player::player::{GameProfile, GameProfileProperty, Player};
use crate::server::player::scoreboard::CREATE_TEAM;
use crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::player_list::player_profile::{GameType, PlayerData, GRAY, GRAY_SIG};
use crate::server::utils::sized_string::SizedString;
use crate::server::world::World;
use crate::utils::seeded_rng::seeded_rng;
use rand::seq::IteratorRandom;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

/// Loads this room's recorded mob data (if any) and spawns one of its observed layouts.
///
/// The scraped data groups spawn entries by `runIndex` - one group per real dungeon run the
/// data was collected from, i.e. one possible mob layout for this room. Rather than spawning
/// the union of everything ever observed (which would over-populate the room), one layout is
/// picked at random and spawned in full, mirroring how the room itself picks one of several
/// possible `RoomData` layouts.
pub fn spawn_room_mobs(world: &mut World, room_index: usize, room: &Room) {
    let Some(file) = get_room_mob_spawns(&room.room_data.name) else { return };

    let mut by_run: BTreeMap<u32, Vec<&MobSpawnJson>> = BTreeMap::new();
    for spawn in &file.spawns {
        by_run.entry(spawn.run_index).or_default().push(spawn);
    }

    let Some(chosen) = by_run.keys().copied().choose(&mut seeded_rng()) else { return };
    let corner = room.get_corner_pos();

    for spawn in &by_run[&chosen] {
        spawn_single_mob(world, room_index, room, corner, spawn);
    }
}

fn spawn_single_mob(world: &mut World, room_index: usize, room: &Room, corner: BlockPos, spawn: &MobSpawnJson) {
    // The archetype (from fullName) is the authoritative source for which model/base kind to
    // spawn; the JSON's own mobType is only consulted as a fallback for unrecognized names,
    // so future/unknown mob names degrade gracefully instead of requiring a code change.
    let archetype = DungeonMobType::from_full_name(&spawn.full_name);
    let base_kind = archetype
        .map(|archetype| archetype.base_kind())
        .or_else(|| MobBaseKind::from_json_str(&spawn.mob_type))
        .unwrap_or(MobBaseKind::Zombie);

    // X/Z are offset from the room's corner (like crypts/superboomwalls/levers), but Y is an
    // offset from the room's own floor (`room_data.bottom`), NOT `corner.y` - the corner's y is
    // always a hardcoded 68 regardless of the room, while `bottom` varies per room (0-66 across
    // the room data), which is what block placement and every other room feature key Y off of.
    let rotated = DVec3::new(spawn.rel_x, spawn.rel_y, spawn.rel_z).rotate(room.rotation); // rotate() never touches y
    let world_pos = DVec3::new(
        corner.x as f64 + rotated.x,
        room.room_data.bottom as f64 + rotated.y,
        corner.z as f64 + rotated.z,
    );
    let yaw = spawn.yaw.rotate(room.rotation);

    let mut equipment = convert_equipment(&spawn.equipment);
    if archetype == Some(DungeonMobType::Sniper) {
        equipment.helmet = Some(sniper_head());
    }
    // Lost Adventurer covers 4 armor variants (Young/Holy/Superior/Unstable Dragon) under one
    // `fullName` - the helmet's own display name is what actually distinguishes which body skin
    // this particular spawn should wear (see `lost_adventurer_skin_for_helmet`).
    let skin_override = (archetype == Some(DungeonMobType::LostAdventurer))
        .then(|| spawn.equipment.get("head").and_then(|head| head.name.as_deref()))
        .flatten()
        .and_then(lost_adventurer_skin_for_helmet);
    // Same format as the reference nametag (§6✰ §cZombie Commander §a3.5M§c❤): the individual
    // mob's own name, star only for actually-starred mobs, and HP only where it's known (rather
    // than inventing a number for archetypes with no confirmed real HP value).
    // OdinClient's `CustomHighlight.starredRegex` (`^(?:.* )?§6✯ .+ .*§c❤$`, confirmed from the
    // mod's raw class-file bytes, not guessed) requires this exact star glyph (U+272F) - a
    // different one (U+2730, "shadowed white star") was used here before, which never matched,
    // so RenderOptimizer's "hide unstarred nametags" treated every mob as unstarred and hid it.
    let star = if spawn.is_starred { "\u{a7}6\u{272F} " } else { "" };
    let health = archetype
        .and_then(|archetype| archetype.base_health())
        .map(|hp| format!(" \u{a7}a{}\u{a7}c\u{2764}", format_health(hp)))
        .unwrap_or_default();
    let nametag = format!("{star}\u{a7}c{}{health}", spawn.full_name);

    let spawn_as_npc = archetype.is_some_and(|archetype| archetype.spawn_as_npc());
    let upside_down = archetype.is_some_and(|archetype| archetype.is_upside_down());

    // Only a *recognized* archetype can actually be tracked to death (an unrecognized mob
    // name gets `NoEntityImpl`, which can never be killed - counting it toward the room's
    // clear requirement would make the room uncheckable forever).
    let counts_toward_clear = spawn.is_starred && archetype.is_some();
    if counts_toward_clear {
        world.server_mut().dungeon.rooms[room_index].starred_mobs_remaining += 1;
    }

    // Fels doesn't start as a live, wandering mob - it lies dormant as an "Enderman head on
    // the floor" marker until a player stumbles right on top of it, per the corrected
    // spec. Everything else spawns active immediately, same as before.
    let full_name = spawn.full_name.clone();

    if archetype == Some(DungeonMobType::Fels) {
        spawn_fels_marker(world, room_index, counts_toward_clear, world_pos, yaw, base_kind, spawn_as_npc, upside_down, equipment, nametag, full_name);
        return;
    }

    let _ = spawn_active_mob(world, room_index, room.entered, counts_toward_clear, world_pos, yaw, archetype, base_kind, spawn_as_npc, upside_down, equipment, nametag, full_name, skin_override);
}

/// Spawns the real, fully-active mob entity (AI, equipment, nametag, combat-state
/// registration) - shared by the normal immediate-spawn path and `FelsMarkerImpl`'s
/// activation once a player triggers it.
///
/// `room_entered` seeds `MobAiState::room_entered` directly (the room's own CURRENT `Room::entered`
/// value at this exact spawn moment, from whichever caller already has it in scope) rather than
/// this function looking it up itself via `world.server_mut().dungeon.rooms.get(room_index)` -
/// that would need a real `Server`/`Dungeon` reachable from every mob spawn, including
/// `perf_bench.rs`'s benchmark harness, which deliberately spawns mobs with `world.server` left
/// null (see that file's own doc comment). Every caller of this function already either has a
/// real `&Room` in scope (the normal room-JSON spawn path) or is triggered by a player physically
/// already being in the room (Crypt Undead/King Midas on superboom, Fels on marker activation) -
/// in both cases the true value is already known for free, no lookup needed.
pub(crate) fn spawn_active_mob(
    world: &mut World,
    room_index: usize,
    room_entered: bool,
    counts_toward_clear: bool,
    world_pos: DVec3,
    yaw: f32,
    archetype: Option<DungeonMobType>,
    base_kind: MobBaseKind,
    spawn_as_npc: bool,
    upside_down: bool,
    equipment: Equipment,
    nametag: String,
    full_name: String,
    skin_override: Option<(&'static str, &'static str)>,
) -> Option<EntityId> {
    // `player`-kind archetypes fall back to a plain zombie model unless explicitly marked
    // `spawn_as_npc` - the real player-NPC path (SpawnPlayer + tab-list skin + hidden nameplate
    // team) is more exotic than the other mob models, so it's opt-in per archetype.
    let variant = if base_kind == MobBaseKind::Player && !spawn_as_npc {
        EntityVariant::Zombie {
            is_child: false,
            is_villager: false,
            is_converting: false,
            is_attacking: false,
        }
    } else {
        match base_kind {
            MobBaseKind::Zombie => EntityVariant::Zombie {
                is_child: false,
                is_villager: false,
                is_converting: false,
                is_attacking: false,
            },
            MobBaseKind::Skeleton => {
                let is_wither = archetype.is_some_and(|archetype| archetype.is_wither_skeleton());
                EntityVariant::Skeleton { is_wither }
            }
            MobBaseKind::Enderman => EntityVariant::Enderman { is_aggressive: false },
            MobBaseKind::Player => EntityVariant::Player,
        }
    };

    let mut metadata = EntityMetadata::new(variant);
    metadata.ai_disabled = true;
    if upside_down {
        // Vanilla's "Dinnerbone"/"Grumm" easter egg: a custom name equal to either string
        // (case-insensitive) flips the entity's model upside-down client-side, independent of
        // whether the name is actually visible. Keep it invisible so no floating text shows -
        // the mob's real name/HP still comes from the following armor-stand nametag below.
        metadata.custom_name = Some("Dinnerbone".to_string());
        metadata.custom_name_visible = false;
    }
    // Otherwise: no vanilla metadata nameplate - every mob gets a following armor-stand
    // nametag instead (see below), matching how the original placeholder zombie-commander
    // spawn looked.

    let spawn_result = if spawn_as_npc {
        let uuid = Uuid::new_v4();
        let username: String = full_name.chars().take(16).collect();
        // Use the per-spawn override if one was passed (currently only Lost Adventurer, whose
        // body skin varies by which of its 4 armor variants this particular spawn is wearing -
        // see `DungeonMobType::lost_adventurer_skin_for_helmet`), else this archetype's fixed
        // skin if one has been set (see `DungeonMobType::skin_override`), else fall back to the
        // same known-good placeholder skin the tab-list system already uses, so these NPCs are
        // guaranteed to render as a normal humanoid instead of risking an invisible/broken model.
        let (texture_value, texture_signature) = skin_override
            .map(|(value, signature)| (value, Some(signature)))
            .or_else(|| archetype.and_then(|archetype| archetype.skin_override()))
            .map(|(value, signature)| (value.to_string(), signature.map(str::to_string)))
            .unwrap_or_else(|| (GRAY.to_string(), Some(GRAY_SIG.to_string())));

        // Register the tab-list profile/skin BEFORE the entity is spawned. Vanilla clients
        // resolve a `SpawnPlayer` packet's skin purely by looking up the UUID in their own
        // tab-list cache - if `SpawnPlayer` arrives first (which `world.spawn_entity_with_uuid`
        // would otherwise do, writing it before `DungeonPlayerMobImpl::spawn`'s
        // `PlayerListItem` in the very same buffer), the client falls back to a blank/unknown
        // profile and the NPC renders invisible instead of with the intended skin.
        register_npc_tab_list_entry(world, uuid, &username, &texture_value, texture_signature.as_deref());

        let entity_impl = DungeonPlayerMobImpl {
            uuid,
            username,
            texture_value,
            texture_signature,
        };
        world.spawn_entity_with_uuid(world_pos, metadata, entity_impl, Some(uuid))
    } else if archetype.is_some() {
        // Known archetype, not an NPC model - drive it with the shared AI pipeline.
        world.spawn_entity(world_pos, metadata, DungeonMobAiImpl)
    } else {
        // Unrecognized mob name (no archetype match) - no behavior, matches how this
        // function already degrades gracefully for anything it doesn't recognize.
        world.spawn_entity(world_pos, metadata, NoEntityImpl)
    };

    let Ok(entity_id) = spawn_result else { return None };

    if let Some((entity, _)) = world.entities.get_mut(&entity_id) {
        entity.yaw = yaw;
    }

    if counts_toward_clear {
        world.entity_starred_mob_room.insert(entity_id, room_index);
    }

    if let Some(archetype) = archetype {
        world.entity_mob_ai.insert(entity_id, MobAiState::new(archetype, world_pos, yaw, room_index, room_entered));

        // Melee archetypes (including `HybridMeleeRanged`'s melee half - Lost/Frozen/Angry
        // Adventurer, Crypt Souleater - confirmed by explicit report: without this they never
        // swung at all, since `attack::try_melee` silently no-ops on a missing cooldown entry)
        // drive their swing/arm-pose through the existing CombatState/AttackCooldown system
        // (already ticked globally by `World::process_combat_state_system`) - it just needs
        // registering here, the same way `spawn_equipped_zombie` already does for the `/spawn`
        // debug command.
        if matches!(profile_for(archetype).attack, AttackModule::Melee { .. } | AttackModule::HybridMeleeRanged { .. }) {
            world.set_combat_state(entity_id, CombatState { aggressive: false, swing_ticks: 0 });
            world.set_attack_cooldown(entity_id, AttackCooldown { ticks: 0 });
            world.set_ai_suspended(entity_id, AISuspended { ticks_left: 10 });
        }
    }

    world.entity_equipment.insert(entity_id, equipment.clone());

    // `entity_equipment` alone only gets picked up when a player later joins/resyncs a chunk
    // (see the view-diff handling in main.rs) - players already in the room when this mob spawns
    // need the EntityEquipment packets queued into the chunk buffer right now, same as
    // `spawn_equipped_zombie` does.
    let chunk_x = (world_pos.x.floor() as i32) >> 4;
    let chunk_z = (world_pos.z.floor() as i32) >> 4;
    if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
        send_equipment_packets(&mut chunk.packet_buffer, entity_id, &equipment);
    }

    // Two separate stands don't work, and neither does swapping the real nametag to a
    // non-ArmorStand species to dodge Skytils' box code - both were tried (Bat, then a baby
    // Zombie) and both got their *text specifically* suppressed while their hitbox still showed
    // in F3+B (confirmed via testing), meaning something on the client hides "fake" mobs by
    // properties (invisible + AI-disabled + unequipped), not by species - and ArmorStand is
    // almost certainly exempted from that since it's the standard vanilla/SkyBlock convention
    // for floating text/props, which is exactly why it's the only thing that reliably renders a
    // name here. So: one ArmorStand, positioned at the real head height Skytils' box needs (raw
    // Y = box's top edge, confirmed from a decompile of `DungeonFeatures.onRenderLivingPre`) -
    // this floats the *text* a bit higher than the offsets used for non-starred mobs below look,
    // since the "small" armor-stand model still adds its own height on top of this raw Y before
    // text renders above it, but a working box takes priority over ideal text placement.
    let is_starred = nametag.starts_with("\u{a7}6\u{272F}");
    let nametag_offset = if is_starred {
        if base_kind == MobBaseKind::Enderman || archetype == Some(DungeonMobType::Withermancer) {
            2.9
        } else {
            1.9
        }
    } else if base_kind == MobBaseKind::Enderman {
        1.9
    } else if archetype == Some(DungeonMobType::Withermancer) {
        1.5
    } else {
        1.1
    };
    let _ = spawn_following_nametag(world, entity_id, &nametag, nametag_offset, EntityVariant::ArmorStand);
    Some(entity_id)
}

/// A Bone (legacy item id 352, `equipment_convert::legacy_item_id`) in the mainhand - matches
/// what Crypt Undead already holds in the scraped room-JSON spawns of this same archetype
/// (`convert_equipment` handles those; this ad-hoc spawn has no JSON entry, so it's built
/// directly here instead).
fn crypt_undead_equipment() -> Equipment {
    Equipment {
        main_hand: Some(ItemStack::new(352)),
        ..Default::default()
    }
}

/// Spawns a Crypt Undead standing at `position` facing `yaw` - called when a player detonates a
/// Crypt with Superboom TNT (see `Dungeon::superboom_at`/`Room::explode_crypt_near`), instead of
/// the crypt secret being granted immediately on explosion. Same archetype, AI, nametag, and
/// skin machinery as the ordinary room-JSON Crypt Undead spawns (`DungeonMobType::CryptUndead`
/// already existed with Dreadlord-shaped mechanics and a real skin) - just triggered ad hoc
/// like `spawn_mimic` is for its own chest-triggered spawn, not tied to any room's starred-mob
/// clear count (`counts_toward_clear: false`) since this isn't a starred mob.
pub fn spawn_crypt_undead(world: &mut World, room_index: usize, position: DVec3, yaw: f32) -> Option<EntityId> {
    let archetype = DungeonMobType::CryptUndead;
    let full_name = "Crypt Undead".to_string();
    let health = archetype.base_health()
        .map(|hp| format!(" \u{a7}a{}\u{a7}c\u{2764}", format_health(hp)))
        .unwrap_or_default();
    let nametag = format!("\u{a7}c{full_name}{health}");

    // `room_entered: true` - only ever triggered by a player detonating a real Crypt right there
    // in the room, so it's unconditionally already entered (see `spawn_active_mob`'s own doc
    // comment for why this is a plain bool, not a lookup).
    spawn_active_mob(
        world,
        room_index,
        true,
        false,
        position,
        yaw,
        Some(archetype),
        archetype.base_kind(),
        archetype.spawn_as_npc(),
        archetype.is_upside_down(),
        crypt_undead_equipment(),
        nametag,
        full_name,
        None,
    )
}

/// An Iron Sword (legacy item id 267) in the mainhand, plus dyed leather boots (301) in a dark
/// purple - per explicit reference (a render of the real mob): worn permanently, visible even
/// while the body itself is invisible (vanilla renders equipment independent of the wearer's own
/// invisibility - no special-case code needed for that half). The RGB is a by-eye estimate off
/// that render (no exact value given) - revisit if a precise value ever turns up.
fn shadow_assassin_equipment() -> Equipment {
    Equipment {
        main_hand: Some(ItemStack::new(267)),
        boots: Some(ItemStack::new(301).leather_rgb(0x4B, 0x00, 0x55)),
        ..Default::default()
    }
}

/// Spawns the Shadow Assassin Champion-room miniboss standing at `position` facing `yaw` -
/// called from `dungeon::room::shadow_assassin::setup` on room entry, same ad-hoc-spawn shape as
/// `spawn_crypt_undead`/`spawn_king_midas` (no captured room-JSON spawn exists for this room -
/// see `DungeonMobType::ShadowAssassin`'s own doc comment). `counts_toward_clear: true` per
/// explicit request - the room completes when he's killed, same "last starred mob dies" path
/// every other starred mob's room uses (`combat::kill_mob` decrements
/// `Room::starred_mobs_remaining`, and at 0 grants any Wither/Blood key plus the usual
/// TNT+50%-blessing room-clear reward - see `Dungeon::grant_room_clear_rewards`). He can already
/// be killed today via the existing lethal-weapon-only path (Hyperion/Spirit Sceptre - see
/// `combat::apply_lethal_hit`) even though no general mob-vs-player damage system exists yet.
pub fn spawn_shadow_assassin(world: &mut World, room_index: usize, room_entered: bool, position: DVec3, yaw: f32) -> Option<EntityId> {
    let archetype = DungeonMobType::ShadowAssassin;
    let full_name = "Shadow Assassin".to_string();
    // Given directly, not the generic star/red-name convention every other archetype's nametag
    // gets from `spawn_active_mob`'s own builder (see `spawn_room_mobs`) - light purple + bold
    // name, no star, per explicit request.
    let health = archetype.base_health()
        .map(|hp| format!(" \u{a7}a{}\u{a7}c\u{2764}", format_health(hp)))
        .unwrap_or_default();
    let nametag = format!("\u{a7}d\u{a7}l{full_name}{health}");

    let mut equipment = shadow_assassin_equipment();
    equipment.no_loot_no_pickup = true;

    let entity_id = spawn_active_mob(
        world,
        room_index,
        room_entered,
        true,
        position,
        yaw,
        Some(archetype),
        archetype.base_kind(),
        archetype.spawn_as_npc(),
        archetype.is_upside_down(),
        equipment,
        nametag,
        full_name,
        None,
    )?;

    // Spawns invisible - per explicit reference, he's constantly invisible except for his
    // permanently-worn boots (which stay visible regardless - see `shadow_assassin_equipment`'s
    // doc comment). `spawn_active_mob` doesn't have a per-archetype invisibility hook today, so
    // this is set directly on the freshly-spawned entity's metadata here instead.
    if let Some((entity, _)) = world.entities.get_mut(&entity_id) {
        entity.metadata.is_invisible = true;
        let metadata = entity.metadata.clone();
        let chunk_x = (position.x.floor() as i32) >> 4;
        let chunk_z = (position.z.floor() as i32) >> 4;
        if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
            chunk.packet_buffer.write_packet(&PacketEntityMetadata {
                entity_id: VarInt(entity_id),
                metadata,
            });
        }
    }

    Some(entity_id)
}

/// Full golden armor + a golden sword - what King Midas wears/wields. Legacy numeric item ids:
/// 283 = golden sword, 314-317 = golden helmet/chestplate/leggings/boots.
fn king_midas_equipment() -> Equipment {
    Equipment {
        main_hand: Some(ItemStack::new(283)),
        helmet: Some(ItemStack::new(314)),
        chest: Some(ItemStack::new(315)),
        legs: Some(ItemStack::new(316)),
        boots: Some(ItemStack::new(317)),
        ..Default::default()
    }
}

/// Spawns King Midas standing at `position` facing `yaw` - called when a player superbooms his
/// golden "crypt" (see `Dungeon::superboom_at`/`Room::explode_kingmidas_near`). Uses the same
/// `spawn_active_mob` pipeline as `spawn_crypt_undead` (player-model NPC, same AI machinery).
/// `counts_toward_clear: true` per explicit request - the room completes when he's killed, same
/// "last starred mob dies" path every other starred mob's room uses (see
/// `spawn_shadow_assassin`'s own doc comment for the full chain); still never registered in
/// `entity_crypt_room` though, so his kill isn't separately credited as a crypt. His actual
/// death sequence (armor breaking off per hit, dying on the 5th) is driven by
/// `ai/combat.rs::apply_king_midas_hit`, not the lethal-weapon-only path other archetypes use -
/// but the 5th hit's `kill_mob` call now drops the usual TNT+50%-blessing reward generically,
/// so `apply_king_midas_weapon_hit` no longer needs its own separate manual TNT drop.
pub fn spawn_king_midas(world: &mut World, room_index: usize, position: DVec3, yaw: f32) -> Option<EntityId> {
    let archetype = DungeonMobType::KingMidas;
    let full_name = "King Midas".to_string();
    // Bold red name + the same green-HP/red-heart suffix every other archetype's nametag uses
    // (see `format_health`) - matches the Mimic's bold-name treatment above.
    let health = archetype.base_health()
        .map(|hp| format!(" \u{a7}a{}\u{a7}c\u{2764}", format_health(hp)))
        .unwrap_or_default();
    let nametag = format!("\u{a7}c\u{a7}l{full_name}{health}");

    // `room_entered: true` - only ever triggered by a player superbooming the room's own real
    // King Midas "crypt" right there, so it's unconditionally already entered (see
    // `spawn_active_mob`'s own doc comment for why this is a plain bool, not a lookup).
    spawn_active_mob(
        world,
        room_index,
        true,
        true,
        position,
        yaw,
        Some(archetype),
        archetype.base_kind(),
        archetype.spawn_as_npc(),
        archetype.is_upside_down(),
        king_midas_equipment(),
        nametag,
        full_name,
        None,
    )
}

/// Base64 Mojang profile "textures" value for the Young Dragon Helmet's own item icon (the
/// skull rendered in inventory/equipment view) - a real capture (`room_data/mobs/atlas.json`,
/// the "Atlas" room's own `runIndex: 2` Lost Adventurer spawn), distinct from the body skin
/// `mob_type::lost_adventurer_skin_for_helmet` resolves for the actual player-model rendering.
const LOST_ADVENTURER_YOUNG_DRAGON_HELMET_ICON: &str = "ewogICJ0aW1lc3RhbXAiIDogMTcyMDA1MDI3MjQwNiwKICAicHJvZmlsZUlkIiA6ICJlMjc5NjliODYyNWY0NDg1YjkyNmM5NTBhMDljMWMwMSIsCiAgInByb2ZpbGVOYW1lIiA6ICJLRVZJTktFTE9LRSIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9hMGU4MWVkMDdkZmIwMjQ0ZDU2ZjRkNWYyYjM3NTUzZWMwMjZmYjQ3OTZmMGZiMGM1N2E4ZWIyNjQ5ODNlMWUwIiwKICAgICAgIm1ldGFkYXRhIiA6IHsKICAgICAgICAibW9kZWwiIDogInNsaW0iCiAgICAgIH0KICAgIH0KICB9Cn0=";

/// Real captured equipment for the "Young Dragon" Lost Adventurer variant
/// (`room_data/mobs/atlas.json`'s `runIndex: 2` entry - dyed color `14542064`, packed RGB
/// `0xDDE4F0`) - hand-built the same way every other ad-hoc archetype's equipment is
/// (`shadow_assassin_equipment`, `king_midas_equipment`, `mimic_equipment`) rather than
/// round-tripping through `equipment_convert::convert_equipment`'s JSON-shaped intermediate.
fn lost_adventurer_young_dragon_equipment() -> Equipment {
    let dyed = |item: i16| ItemStack::new(item).leather_rgb(0xDD, 0xE4, 0xF0);

    let mut helmet = ItemStack {
        item: 397, // skull
        stack_size: 1,
        metadata: 3, // player head - resolves its icon texture from the embedded SkullOwner
        tag_compound: None,
    };
    helmet.set_skull_owner(LOST_ADVENTURER_YOUNG_DRAGON_HELMET_ICON);
    helmet.set_display_name("\u{a7}6Young Dragon Helmet");
    helmet.set_unbreakable(true);

    let mut chest = dyed(299);
    chest.set_display_name("\u{a7}6Young Dragon Chestplate");
    chest.set_unbreakable(true);

    let mut legs = dyed(300);
    legs.set_display_name("\u{a7}6Young Dragon Leggings");
    legs.set_unbreakable(true);

    let mut boots = dyed(301);
    boots.set_display_name("\u{a7}6Young Dragon Boots");
    boots.set_unbreakable(true);

    let mut sword = ItemStack::new(276); // diamond sword
    sword.set_display_name("\u{a7}6Aspect of the Dragons");
    sword.set_unbreakable(true);

    Equipment {
        main_hand: Some(sword),
        helmet: Some(helmet),
        chest: Some(chest),
        legs: Some(legs),
        boots: Some(boots),
        no_loot_no_pickup: true,
        unbreakable: true,
    }
}

/// Spawns a Lost Adventurer (Young Dragon variant - "any type" per explicit request, picked
/// arbitrarily among the 4 real variants) standing at `position` facing `yaw` - called from
/// `dungeon::room::default_room::setup` on room entry for the "Default" room, which always gets
/// exactly one this way regardless of which of its two captured mob-data layouts
/// (`spawn_room_mobs`) happens to load, since only one of those two real captures includes him.
/// Nametag matches the real starred-mob convention (star + red name + HP) - the real captured
/// `room_data/mobs/default.json` entry for him is `isStarred: true`, same as this one.
/// `counts_toward_clear: true`, same reasoning as Shadow Assassin/King Midas - see
/// `spawn_shadow_assassin`'s own doc comment for the full room-clear/reward chain.
pub fn spawn_lost_adventurer(world: &mut World, room_index: usize, room_entered: bool, position: DVec3, yaw: f32) -> Option<EntityId> {
    let archetype = DungeonMobType::LostAdventurer;
    let full_name = "Lost Adventurer".to_string();
    let health = archetype.base_health()
        .map(|hp| format!(" \u{a7}a{}\u{a7}c\u{2764}", format_health(hp)))
        .unwrap_or_default();
    let nametag = format!("\u{a7}6\u{272F} \u{a7}c{full_name}{health}");

    let skin_override = lost_adventurer_skin_for_helmet("Young Dragon Helmet");

    spawn_active_mob(
        world,
        room_index,
        room_entered,
        true,
        position,
        yaw,
        Some(archetype),
        archetype.base_kind(),
        archetype.spawn_as_npc(),
        archetype.is_upside_down(),
        lost_adventurer_young_dragon_equipment(),
        nametag,
        full_name,
        skin_override,
    )
}

/// Base64 Mojang profile "textures" value for the Frozen Blaze Helmet's own item icon (a real
/// capture, `room_data/mobs/cathedral.json`'s Frozen Adventurer entry), distinct from the body
/// skin `DungeonMobType::skin_override` already resolves for this archetype.
const FROZEN_BLAZE_HELMET_ICON: &str = "ewogICJ0aW1lc3RhbXAiIDogMTY2MTA4NTAzNTkyNywKICAicHJvZmlsZUlkIiA6ICJjMTNkYzkxZjg1YjA0ZWM4OGU2NDk5YzdjZDc4Zjk3MSIsCiAgInByb2ZpbGVOYW1lIiA6ICJjYXNzdGhlY3J5cHRpZCIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS83MDMxMGI1NWVlZDk1OGZlZThmZmFhZjcxODQ2NzE2N2RlYjFhN2M5MDQ2ZDY3YjI1ODg3YjU5NjkyNTYzNmJiIgogICAgfQogIH0KfQ==";

/// Real captured equipment for Frozen Adventurer (`room_data/mobs/cathedral.json`'s own
/// `runIndex: 1` entry - dyed color `10541807`, packed RGB `0xA0DAEF`) - hand-built the same way
/// as `lost_adventurer_young_dragon_equipment` above.
fn frozen_adventurer_equipment() -> Equipment {
    let dyed = |item: i16| ItemStack::new(item).leather_rgb(0xA0, 0xDA, 0xEF);

    let mut helmet = ItemStack {
        item: 397, // skull
        stack_size: 1,
        metadata: 3, // player head - resolves its icon texture from the embedded SkullOwner
        tag_compound: None,
    };
    helmet.set_skull_owner(FROZEN_BLAZE_HELMET_ICON);
    helmet.set_display_name("\u{a7}6Frozen Blaze Helmet");
    helmet.set_unbreakable(true);

    let mut chest = dyed(299);
    chest.set_display_name("\u{a7}6Frozen Blaze Chestplate");
    chest.set_unbreakable(true);

    let mut legs = dyed(300);
    legs.set_display_name("\u{a7}6Frozen Blaze Leggings");
    legs.set_unbreakable(true);

    let mut boots = dyed(301);
    boots.set_display_name("\u{a7}6Frozen Blaze Boots");
    boots.set_unbreakable(true);

    let mut wand = ItemStack::new(280); // stick
    wand.set_display_name("\u{a7}9Ice Spray Wand");
    wand.set_unbreakable(true);

    Equipment {
        main_hand: Some(wand),
        helmet: Some(helmet),
        chest: Some(chest),
        legs: Some(legs),
        boots: Some(boots),
        no_loot_no_pickup: true,
        unbreakable: true,
    }
}

/// Spawns a Frozen Adventurer standing at `position` facing `yaw` - same ad-hoc shape as
/// `spawn_lost_adventurer`. `counts_toward_clear: true`, same reasoning as every other ad-hoc
/// miniboss - see `spawn_shadow_assassin`'s own doc comment for the full room-clear/reward chain.
/// Nametag is starred, matching the real captured entry's own `isStarred: true`.
pub fn spawn_frozen_adventurer(world: &mut World, room_index: usize, room_entered: bool, position: DVec3, yaw: f32) -> Option<EntityId> {
    let archetype = DungeonMobType::FrozenAdventurer;
    let full_name = "Frozen Adventurer".to_string();
    let health = archetype.base_health()
        .map(|hp| format!(" \u{a7}a{}\u{a7}c\u{2764}", format_health(hp)))
        .unwrap_or_default();
    let nametag = format!("\u{a7}6\u{272F} \u{a7}c{full_name}{health}");

    spawn_active_mob(
        world,
        room_index,
        room_entered,
        true,
        position,
        yaw,
        Some(archetype),
        archetype.base_kind(),
        archetype.spawn_as_npc(),
        archetype.is_upside_down(),
        frozen_adventurer_equipment(),
        nametag,
        full_name,
        None, // falls back to `DungeonMobType::skin_override()`, already set for this archetype
    )
}

/// Real captured equipment for Angry Archaeologist (`room_data/mobs/atlas.json`'s own
/// `runIndex: 1` entry) - full diamond armor, no dye (unlike the two leather-armor archetypes
/// above).
fn angry_archaeologist_equipment() -> Equipment {
    let mut helmet = ItemStack::new(310); // diamond helmet
    helmet.set_display_name("\u{a7}6Perfect Helmet - Tier XII");
    helmet.set_unbreakable(true);

    let mut chest = ItemStack::new(311); // diamond chestplate
    chest.set_display_name("\u{a7}6Perfect Chestplate - Tier XII");
    chest.set_unbreakable(true);

    let mut legs = ItemStack::new(312); // diamond leggings
    legs.set_display_name("\u{a7}6Perfect Leggings - Tier XII");
    legs.set_unbreakable(true);

    let mut boots = ItemStack::new(313); // diamond boots
    boots.set_display_name("\u{a7}6Perfect Boots - Tier XII");
    boots.set_unbreakable(true);

    let mut sword = ItemStack::new(276); // diamond sword
    sword.set_display_name("\u{a7}aDiamond Sword");
    sword.set_unbreakable(true);

    Equipment {
        main_hand: Some(sword),
        helmet: Some(helmet),
        chest: Some(chest),
        legs: Some(legs),
        boots: Some(boots),
        no_loot_no_pickup: true,
        unbreakable: true,
    }
}

/// Spawns an Angry Archaeologist standing at `position` facing `yaw` - same ad-hoc shape as
/// `spawn_lost_adventurer`. `counts_toward_clear: true`, same reasoning as every other ad-hoc
/// miniboss - see `spawn_shadow_assassin`'s own doc comment for the full room-clear/reward chain.
/// Nametag is unstarred, matching the real captured entry's own `isStarred: false`.
pub fn spawn_angry_archaeologist(world: &mut World, room_index: usize, room_entered: bool, position: DVec3, yaw: f32) -> Option<EntityId> {
    let archetype = DungeonMobType::AngryArchaeologist;
    let full_name = "Angry Archaeologist".to_string();
    let health = archetype.base_health()
        .map(|hp| format!(" \u{a7}a{}\u{a7}c\u{2764}", format_health(hp)))
        .unwrap_or_default();
    let nametag = format!("\u{a7}c{full_name}{health}");

    spawn_active_mob(
        world,
        room_index,
        room_entered,
        true,
        position,
        yaw,
        Some(archetype),
        archetype.base_kind(),
        archetype.spawn_as_npc(),
        archetype.is_upside_down(),
        angry_archaeologist_equipment(),
        nametag,
        full_name,
        None, // falls back to `DungeonMobType::skin_override()`, already set for this archetype
    )
}

/// Base64 Mojang profile "textures" value for the mimic's head - a `minecraft:player_head`
/// skull, same mechanism as `fels_marker_head` below.
const MIMIC_SKULL_TEXTURE: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvZTE5YzEyNTQzYmM3NzkyNjA1ZWY2OGUxZjg3NDlhZThmMmEzODFkOTA4NWQ0ZDRiNzgwYmExMjgyZDM1OTdhMCJ9fX0=";

/// Leather chestplate/leggings/boots dyed hex `dbcc8f`, plus the skull above for the helmet -
/// matches real Hypixel's actual mimic appearance (a disguised baby zombie), not a bare mob.
fn mimic_equipment() -> Equipment {
    let dyed = |item: i16| ItemStack::new(item).leather_rgb(0xdb, 0xcc, 0x8f);

    let mut helmet = ItemStack {
        item: 397, // skull
        stack_size: 1,
        metadata: 3, // player head - resolves its texture from the embedded SkullOwner
        tag_compound: None,
    };
    helmet.set_skull_owner(MIMIC_SKULL_TEXTURE);

    Equipment {
        helmet: Some(helmet),
        chest: Some(dyed(299)),
        legs: Some(dyed(300)),
        boots: Some(dyed(301)),
        ..Default::default()
    }
}

/// Spawns the mimic mob at `position` facing `yaw` - called when a player opens the dungeon's
/// one designated mimic chest (see `main.rs`'s post-locked-chest-spawn selection step and
/// `BlockInteractAction::MimicChest`). A disguised baby zombie driven by the same AI pipeline
/// as room-spawned mobs (`DungeonMobAiImpl`/`Mimic`'s basic-melee `AiProfile`), just spawned
/// ad hoc instead of from room JSON - not tied to any room's starred-mob clear count.
///
/// Note: Odin's `onEntityDeath` mimic-kill heuristic requires armor slots 0-3 (held item/
/// boots/leggings/chestplate) to all be empty - this mimic doesn't satisfy that, since it's
/// equipped to match real Hypixel's actual appearance instead of staying bare for that one
/// mod's detection. Confirmed trade-off, not an oversight.
pub fn spawn_mimic(world: &mut World, position: DVec3, yaw: f32, room_index: usize) -> anyhow::Result<EntityId> {
    let metadata = EntityMetadata::new(EntityVariant::Zombie {
        is_child: true,
        is_villager: false,
        is_converting: false,
        is_attacking: false,
    });

    let entity_id = world.spawn_entity(position, metadata, DungeonMobAiImpl)?;

    if let Some((entity, _)) = world.entities.get_mut(&entity_id) {
        entity.yaw = yaw;
    }

    // Always `true` - a mimic only ever spawns from a chest a player just physically opened,
    // so the room is unconditionally already entered at this exact moment (see
    // `spawn_active_mob`'s own doc comment for why this is a plain bool, not a lookup).
    world.entity_mob_ai.insert(entity_id, MobAiState::new(DungeonMobType::Mimic, position, yaw, room_index, true));
    world.set_combat_state(entity_id, CombatState { aggressive: false, swing_ticks: 0 });
    world.set_attack_cooldown(entity_id, AttackCooldown { ticks: 0 });
    world.set_ai_suspended(entity_id, AISuspended { ticks_left: 10 });

    let equipment = mimic_equipment();
    let chunk_x = (position.x.floor() as i32) >> 4;
    let chunk_z = (position.z.floor() as i32) >> 4;
    if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
        send_equipment_packets(&mut chunk.packet_buffer, entity_id, &equipment);
    }
    world.entity_equipment.insert(entity_id, equipment);

    let health = DungeonMobType::Mimic.base_health()
        .map(|hp| format!(" \u{a7}a{}\u{a7}c\u{2764}", format_health(hp)))
        .unwrap_or_default();
    let nametag = format!("\u{a7}8[\u{a7}7Lv115\u{a7}8]\u{a7}c\u{a7}lMimic{health}");
    // Roughly half the standard 1.1 head-height offset used elsewhere (see the comment on
    // that one), matching the mimic's `is_child` baby-zombie scale (~half an adult's height).
    let _ = spawn_following_nametag(world, entity_id, &nametag, 0.55, EntityVariant::ArmorStand);

    Ok(entity_id)
}

/// Broadcasts an `ADD_PLAYER` tab-list entry (with skin) for `uuid` to every currently
/// connected player, so it's already known client-side before the corresponding
/// `SpawnPlayer` packet is sent (see call site for why the order matters). Players who join
/// later still get it fresh via `DungeonPlayerMobImpl::spawn`'s own registration when they
/// first view the mob's chunk.
pub(crate) fn register_npc_tab_list_entry(world: &mut World, uuid: Uuid, username: &str, texture_value: &str, texture_signature: Option<&str>) {
    let mut properties = HashMap::new();
    properties.insert("textures".to_string(), GameProfileProperty {
        value: texture_value.to_string(),
        signature: texture_signature.map(str::to_string),
    });

    let player_data = PlayerData {
        ping: 0,
        game_mode: GameType::Survival,
        profile: GameProfile {
            uuid,
            username: username.to_string(),
            properties,
        },
        display_name: Some(ChatComponentTextBuilder::new(username.to_string()).build()),
    };

    let packet = PlayerListItem {
        action: VarInt(0), // ADD_PLAYER
        players: vec![&player_data],
    };
    for player in world.players.values_mut() {
        player.write_packet(&packet);
    }
}

/// Entity implementation for archetypes spawned as real player-model NPCs (Crypt Dreadlord,
/// Zombie Commander, etc). Player entities must be registered in the tab list with a skin
/// before a `SpawnPlayer` packet will render correctly, and their default nameplate (bound to
/// their profile username) can't be styled - so it's hidden via a scoreboard team named after
/// this specific entity's id (never shared with any other entity, unlike Mort's NPC code which
/// reuses one literal team name and hits "team already exists" once more than one is added).
struct DungeonPlayerMobImpl {
    uuid: Uuid,
    username: String,
    texture_value: String,
    texture_signature: Option<String>,
}

impl EntityImpl for DungeonPlayerMobImpl {
    fn spawn(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        let mut properties = HashMap::new();
        properties.insert("textures".to_string(), GameProfileProperty {
            value: self.texture_value.clone(),
            signature: self.texture_signature.clone(),
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

        let team_name = format!("dmob{}", entity.id);
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

    fn tick(&mut self, entity: &mut Entity, buffer: &mut PacketBuffer) {
        run_mob_ai(entity, buffer);
    }

    fn interact(&mut self, entity: &mut Entity, player: &mut Player, action: &EntityInteractionType) -> bool {
        if *action == EntityInteractionType::Attack {
            if !crate::server::entity::dungeon_mobs::ai::combat::apply_king_midas_hit(entity, player) {
                crate::server::entity::dungeon_mobs::ai::combat::apply_lethal_hit(entity, player);
            }
            crate::server::entity::dungeon_mobs::ai::aggro::on_mob_attacked(entity, player.client_id);
            crate::server::entity::dungeon_mobs::ai::combat::on_player_damaged_mob(entity.world_mut(), entity.id);
        }
        false
    }
}

/// How close a player needs to get to the Fels marker before it activates.
const FELS_MARKER_ACTIVATION_RADIUS: f64 = 5.0;

/// Visually lowers the marker armor stand so its head slot (the "Enderman head") sits near
/// floor level instead of at a normal armor stand's head height (~1.6 blocks up) - a rough
/// approximation of "lying on the floor" without needing dedicated ArmorStand pose/NBT
/// support, which this codebase doesn't have yet.
const FELS_MARKER_Y_OFFSET: f64 = -1.5;

/// Vanilla has no real "Enderman head" skull item (only skeleton/wither skeleton/zombie/
/// player/creeper exist) - this uses a player-head skull (metadata 3) with an embedded skin
/// texture for `MHF_Enderman`, a long-standing special Mojang account whose skin *is* an
/// Enderman face (the same trick servers/plugins have used for years for "mob head" decor
/// items - MHF_Zombie, MHF_Blaze, etc). A plain owner *name* alone (no embedded texture) does
/// NOT work here - that live name->skin lookup is something Bukkit-style server plugins
/// perform server-side before ever sending the item; a vanilla client never does it itself,
/// so without an embedded texture it just falls back to a default Steve/Alex skin (exactly
/// what was seen). The value below is the actual current texture property fetched directly
/// from Mojang's session server for MHF_Enderman's UUID (40ffb372-12f6-4678-b3f2-2176bf56dd4b),
/// not guessed.
const MHF_ENDERMAN_SKIN_TEXTURE: &str = "ewogICJ0aW1lc3RhbXAiIDogMTc4MzI4NzUyNTQ4NCwKICAicHJvZmlsZUlkIiA6ICI0MGZmYjM3MjEyZjY0Njc4YjNmMjIxNzZiZjU2ZGQ0YiIsCiAgInByb2ZpbGVOYW1lIiA6ICJNSEZfRW5kZXJtYW4iLAogICJzaWduYXR1cmVSZXF1aXJlZCIgOiB0cnVlLAogICJ0ZXh0dXJlcyIgOiB7CiAgICAiU0tJTiIgOiB7CiAgICAgICJ1cmwiIDogImh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvMWIwOWEzNzUyNTEwZTkxNGIwYmRjOTA5NmIzOTJiYjM1OWY3YThlOGE5NTY2YTAyZTdmNjZmYWZmOGQ2Zjg5ZSIKICAgIH0KICB9Cn0=";

fn fels_marker_head() -> ItemStack {
    let mut stack = ItemStack {
        item: 397, // skull
        stack_size: 1,
        metadata: 3, // player head - resolves its texture from the embedded SkullOwner
        tag_compound: None,
    };
    stack.set_skull_owner(MHF_ENDERMAN_SKIN_TEXTURE);
    stack
}

const SNIPER_SKULL_TEXTURE: &str = "eyJ0ZXh0dXJlcyI6eyJTS0lOIjp7InVybCI6Imh0dHA6Ly90ZXh0dXJlcy5taW5lY3JhZnQubmV0L3RleHR1cmUvYjE4YzA3MWYwODBkYmE1MGE2MmE2MjYzZmY3MjRlZGMxNTdjZTRmYjQ4ODNjY2VmZjI0OTFkNWJiZGU4MzBjMSJ9fX0=";

fn sniper_head() -> ItemStack {
    let mut stack = ItemStack {
        item: 397, // skull
        stack_size: 1,
        metadata: 3, // player head - resolves its texture from the embedded SkullOwner
        tag_compound: None,
    };
    stack.set_skull_owner(SNIPER_SKULL_TEXTURE);
    stack
}

/// Spawns Fels's inactive marker: an invisible armor stand wearing a skull (see
/// `fels_marker_head`), doing nothing but watching for a nearby player. `FelsMarkerImpl`
/// carries everything `spawn_active_mob` needs so activation can call it later exactly as if
/// this had been a normal immediate spawn.
fn spawn_fels_marker(
    world: &mut World,
    room_index: usize,
    counts_toward_clear: bool,
    world_pos: DVec3,
    yaw: f32,
    base_kind: MobBaseKind,
    spawn_as_npc: bool,
    upside_down: bool,
    equipment: Equipment,
    nametag: String,
    full_name: String,
) {
    let mut metadata = EntityMetadata::new(EntityVariant::ArmorStand);
    metadata.is_invisible = true;

    let marker_pos = DVec3::new(world_pos.x, world_pos.y + FELS_MARKER_Y_OFFSET, world_pos.z);
    let marker_impl = FelsMarkerImpl {
        trigger_pos: world_pos,
        room_index,
        counts_toward_clear,
        yaw,
        base_kind,
        spawn_as_npc,
        upside_down,
        equipment,
        nametag,
        full_name,
    };

    let Ok(entity_id) = world.spawn_entity(marker_pos, metadata, marker_impl) else { return };

    let head_equipment = Equipment { helmet: Some(fels_marker_head()), ..Default::default() };
    let chunk_x = (marker_pos.x.floor() as i32) >> 4;
    let chunk_z = (marker_pos.z.floor() as i32) >> 4;
    if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
        send_equipment_packets(&mut chunk.packet_buffer, entity_id, &head_equipment);
    }
    world.entity_equipment.insert(entity_id, head_equipment);
}

/// Inert "Enderman head lying on the floor" marker for Fels. Fels doesn't visibly wander as a
/// live mob until a player stumbles right on top of its spawn point (see the corrected spec
/// this replaced the old immediate-spawn behavior with) - this does nothing except check
/// distance to players each tick; once one gets within `FELS_MARKER_ACTIVATION_RADIUS`, it
/// despawns itself and hands off to `spawn_active_mob` at the real trigger position.
struct FelsMarkerImpl {
    /// Where the real Fels should spawn - the marker's own `entity.position` is offset lower
    /// for the "lying on the floor" visual, so this is tracked separately.
    trigger_pos: DVec3,
    room_index: usize,
    counts_toward_clear: bool,
    yaw: f32,
    base_kind: MobBaseKind,
    spawn_as_npc: bool,
    upside_down: bool,
    equipment: Equipment,
    nametag: String,
    full_name: String,
}

impl EntityImpl for FelsMarkerImpl {
    fn tick(&mut self, entity: &mut Entity, _packet_buffer: &mut PacketBuffer) {
        let world = entity.world_mut();
        let triggered = world.players.values().any(|player| {
            player.position.distance_to(&self.trigger_pos) <= FELS_MARKER_ACTIVATION_RADIUS
        });
        if !triggered {
            return;
        }

        let entity_id: EntityId = entity.id;
        world.despawn_entity(entity_id);
        // `room_entered: true` - only ever triggered by a real player standing within
        // `FELS_MARKER_ACTIVATION_RADIUS`, so the room is unconditionally already entered (see
        // `spawn_active_mob`'s own doc comment for why this is a plain bool, not a lookup).
        let _ = spawn_active_mob(
            world,
            self.room_index,
            true,
            self.counts_toward_clear,
            self.trigger_pos,
            self.yaw,
            Some(DungeonMobType::Fels),
            self.base_kind,
            self.spawn_as_npc,
            self.upside_down,
            self.equipment.clone(),
            self.nametag.clone(),
            self.full_name.clone(),
            None,
        );
    }
}

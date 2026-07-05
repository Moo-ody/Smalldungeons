//! Spawns dungeon mobs for a room from its recorded mob data (see `spawn_data.rs`).
//!
//! This is the only place that turns JSON spawn/placement entries into actual entities -
//! everything else in `dungeon_mobs` just describes archetypes or converts data. No mob gets
//! a bespoke struct here: every entry is spawned through the same handful of code paths,
//! parameterized by its [`DungeonMobType`]/[`MobBaseKind`] and JSON equipment/position.

use crate::dungeon::room::room::Room;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{PlayerListItem, Teams};
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::net::var_int::VarInt;
use crate::server::block::block_position::BlockPos;
use crate::server::block::rotatable::Rotatable;
use crate::server::entity::dungeon_mobs::ai::attack::AttackModule;
use crate::server::entity::dungeon_mobs::ai::profile::profile_for;
use crate::server::entity::dungeon_mobs::ai::state::MobAiState;
use crate::server::entity::dungeon_mobs::ai::{run_mob_ai, DungeonMobAiImpl};
use crate::server::entity::dungeon_mobs::equipment_convert::convert_equipment;
use crate::server::entity::dungeon_mobs::mob_type::{format_health, DungeonMobType, MobBaseKind};
use crate::server::entity::dungeon_mobs::spawn_data::{get_room_mob_spawns, MobSpawnJson};
use crate::server::entity::entity::{Entity, EntityId, EntityImpl, NoEntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::entity::equipment::Equipment;
use crate::server::entity::spawn_equipped::{send_equipment_packets, spawn_following_nametag, AISuspended, AttackCooldown, CombatState};
use crate::server::items::item_stack::ItemStack;
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

    let equipment = convert_equipment(&spawn.equipment);
    // Same format as the reference nametag (§6✰ §cZombie Commander §a3.5M§c❤): the individual
    // mob's own name, star only for actually-starred mobs, and HP only where it's known (rather
    // than inventing a number for archetypes with no confirmed real HP value).
    let star = if spawn.is_starred { "\u{a7}6\u{2730} " } else { "" };
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

    spawn_active_mob(world, room_index, counts_toward_clear, world_pos, yaw, archetype, base_kind, spawn_as_npc, upside_down, equipment, nametag, full_name);
}

/// Spawns the real, fully-active mob entity (AI, equipment, nametag, combat-state
/// registration) - shared by the normal immediate-spawn path and `FelsMarkerImpl`'s
/// activation once a player triggers it.
fn spawn_active_mob(
    world: &mut World,
    room_index: usize,
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
) {
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
        // No per-archetype skin has been chosen yet (see `DungeonMobType::skin_override`) -
        // fall back to the same known-good placeholder skin the tab-list system already uses,
        // so these NPCs are guaranteed to render as a normal humanoid instead of risking an
        // invisible/broken model until real skins are filled in.
        let (texture_value, texture_signature) = archetype
            .and_then(|archetype| archetype.skin_override())
            .map(|(value, signature)| (value.to_string(), signature.to_string()))
            .unwrap_or_else(|| (GRAY.to_string(), GRAY_SIG.to_string()));

        // Register the tab-list profile/skin BEFORE the entity is spawned. Vanilla clients
        // resolve a `SpawnPlayer` packet's skin purely by looking up the UUID in their own
        // tab-list cache - if `SpawnPlayer` arrives first (which `world.spawn_entity_with_uuid`
        // would otherwise do, writing it before `DungeonPlayerMobImpl::spawn`'s
        // `PlayerListItem` in the very same buffer), the client falls back to a blank/unknown
        // profile and the NPC renders invisible instead of with the intended skin.
        register_npc_tab_list_entry(world, uuid, &username, &texture_value, &texture_signature);

        let entity_impl = DungeonPlayerMobImpl {
            uuid,
            username,
            texture_value,
            texture_signature: Some(texture_signature),
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

    let Ok(entity_id) = spawn_result else { return };

    if let Some((entity, _)) = world.entities.get_mut(&entity_id) {
        entity.yaw = yaw;
    }

    if counts_toward_clear {
        world.entity_starred_mob_room.insert(entity_id, room_index);
    }

    if let Some(archetype) = archetype {
        world.entity_mob_ai.insert(entity_id, MobAiState::new(archetype, world_pos, yaw));

        // Melee archetypes drive their swing/arm-pose through the existing
        // CombatState/AttackCooldown system (already ticked globally by
        // `World::process_combat_state_system`) - it just needs registering here, the
        // same way `spawn_equipped_zombie` already does for the `/spawn` debug command.
        if matches!(profile_for(archetype).attack, AttackModule::Melee { .. }) {
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

    // Enderman is considerably taller than the other mob models, so its nametag needs a
    // bigger offset to clear its head instead of floating at chest height.
    let nametag_offset = if base_kind == MobBaseKind::Enderman { 1.0 } else { 0.1 };
    let _ = spawn_following_nametag(world, entity_id, &nametag, nametag_offset);
}

/// Broadcasts an `ADD_PLAYER` tab-list entry (with skin) for `uuid` to every currently
/// connected player, so it's already known client-side before the corresponding
/// `SpawnPlayer` packet is sent (see call site for why the order matters). Players who join
/// later still get it fresh via `DungeonPlayerMobImpl::spawn`'s own registration when they
/// first view the mob's chunk.
fn register_npc_tab_list_entry(world: &mut World, uuid: Uuid, username: &str, texture_value: &str, texture_signature: &str) {
    let mut properties = HashMap::new();
    properties.insert("textures".to_string(), GameProfileProperty {
        value: texture_value.to_string(),
        signature: Some(texture_signature.to_string()),
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

    fn interact(&mut self, entity: &mut Entity, player: &mut Player, action: &EntityInteractionType) {
        if *action == EntityInteractionType::Attack {
            crate::server::entity::dungeon_mobs::ai::combat::apply_lethal_hit(entity, player);
            crate::server::entity::dungeon_mobs::ai::aggro::on_mob_attacked(entity, player.client_id);
        }
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
        spawn_active_mob(
            world,
            self.room_index,
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
        );
    }
}

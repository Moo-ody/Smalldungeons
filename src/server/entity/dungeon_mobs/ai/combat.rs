//! Lets dungeon mobs actually be killed. There is no general mob HP/damage system anywhere
//! in this codebase (mob nametags show a *display* HP from `DungeonMobType::base_health`,
//! but nothing decrements it) - building a full damage model is out of scope here, so this
//! wires an explicit, simple rule instead: hitting a dungeon mob with either of the two
//! requested endgame weapons kills it outright.
//!
//! `kill_mob` (death animation + species-specific sound + despawn) is shared with
//! `hyperion::handle_hyperion_explosion`'s AOE kill path - both Hyperion and Spirit Sceptre
//! (whose bat projectile calls into that same function) route through here. King Midas is the
//! one exception to "instant kill": `apply_king_midas_weapon_hit` intercepts every weapon
//! (melee, Hyperion, Spirit Sceptre alike) and turns it into one step of his armor-break/
//! 5-hit-kill sequence instead - see that function's doc comment.

use crate::net::protocol::play::clientbound::{EntityEquipment, EntityStatus, PacketEntityMetadata, SoundEffect};
use crate::net::var_int::VarInt;
use crate::server::entity::dungeon_mobs::mob_type::{format_health, DungeonMobType};
use crate::server::entity::entity::{Entity, EntityId, NoEntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::items::Item;
use crate::server::player::inventory::ItemSlot;
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;

/// `EntityStatus` code for the generic "entity died" animation - the client plays the usual
/// death animation/particles for whatever entity type this is on its own.
const ENTITY_DEATH_STATUS: i8 = 3;

/// Whether `player`'s currently-held item should instantly kill any dungeon mob it hits.
fn wields_lethal_weapon(player: &Player) -> bool {
    matches!(
        player.inventory.get_hotbar_slot(player.held_slot as usize),
        Some(ItemSlot::Filled(Item::Hyperion | Item::SpiritSceptre, _))
    )
}

/// Kills `entity` outright if `player` hit it while holding a lethal weapon.
pub fn apply_lethal_hit(entity: &mut Entity, player: &Player) {
    if !wields_lethal_weapon(player) {
        return;
    }
    let entity_id = entity.id;
    kill_mob(entity.world_mut(), entity_id);
}

/// How many hits King Midas takes before he dies - one per armor piece (helmet, chestplate,
/// leggings, boots), then a 5th that actually kills him.
const KING_MIDAS_HITS_TO_KILL: u8 = 5;

/// `EntityImpl::interact` entry point for King Midas's hit mechanic - thin wrapper around
/// `apply_king_midas_weapon_hit` for callers that only have an `&mut Entity` (the melee-attack
/// interact hooks). `_player` is unused: unlike `apply_lethal_hit`, ANY weapon counts here, not
/// just Hyperion/Spirit Sceptre - it's only kept so the two functions share a call signature at
/// their shared call sites (`spawner.rs`/`ai/mod.rs`).
pub fn apply_king_midas_hit(entity: &mut Entity, _player: &Player) -> bool {
    let entity_id = entity.id;
    apply_king_midas_weapon_hit(entity.world_mut(), entity_id)
}

/// King Midas's bespoke "hit" mechanic (see `spawner::spawn_king_midas`), keyed off just an
/// `EntityId` so `hyperion::handle_hyperion_explosion`'s AOE path (shared by Spirit Sceptre's
/// bat projectile) can call it too, not just the direct melee-attack interact hooks. Each of
/// his first 4 hits plays an anvil-break sound, strips one piece of his golden armor, and
/// updates his nametag's HP suffix to `base_health / KING_MIDAS_HITS_TO_KILL` less than before;
/// the 5th kills him and drops a Superboom TNT pickup, the same "cosmetic prop next to a body"
/// mechanism the Wither/Blood door key companion TNT uses (`Dungeon::maybe_grant_door_key`/
/// `spawn_pickup`). Returns `false` for every other archetype (or an already-gone entity) so
/// callers fall through to their normal instant-kill path instead.
pub fn apply_king_midas_weapon_hit(world: &mut World, entity_id: EntityId) -> bool {
    let is_king_midas = world.entity_mob_ai.get(&entity_id)
        .is_some_and(|state| state.archetype == DungeonMobType::KingMidas);
    if !is_king_midas {
        return false;
    }
    let Some(pos) = world.entities.get(&entity_id).map(|(entity, _)| entity.position) else {
        return false;
    };

    let hits = world.entity_king_midas_hits.entry(entity_id).or_insert(0);
    *hits += 1;
    let hit_count = *hits;

    if hit_count >= KING_MIDAS_HITS_TO_KILL {
        world.entity_king_midas_hits.remove(&entity_id);
        kill_mob(world, entity_id);
        crate::dungeon::dungeon::spawn_pickup(world, pos, crate::dungeon::room::secrets::PickupKind::Tnt);
        return true;
    }

    for player in world.players.values_mut() {
        // `entity.item.break`, not an anvil - per explicit correction: golden armor snapping off
        // sounds like a broken item/tool, not an anvil falling apart. Volume raised above the
        // normal 1.0 per explicit follow-up request ("make it louder") - Minecraft's sound volume
        // also extends how far the sound carries before falloff, not just perceived loudness up
        // close.
        player.write_packet(&SoundEffect {
            sound: Sounds::ItemBreak.id(),
            pos_x: pos.x,
            pos_y: pos.y,
            pos_z: pos.z,
            volume: 3.0,
            pitch: 1.0,
        });
    }
    strip_next_king_midas_armor_piece(world, entity_id, hit_count);
    update_king_midas_nametag_health(world, entity_id, hit_count);
    true
}

/// Recomputes King Midas's displayed HP as `base_health * (KING_MIDAS_HITS_TO_KILL - hit_count)
/// / KING_MIDAS_HITS_TO_KILL` - i.e. each hit knocks off an even fifth of his starting HP - and
/// pushes the new nametag text to every connected player. His nametag is a separate following
/// armor-stand entity (see `spawn_following_nametag`), found via `entity_following_nametag`.
fn update_king_midas_nametag_health(world: &mut World, entity_id: EntityId, hit_count: u8) {
    let Some(&nametag_id) = world.entity_following_nametag.get(&entity_id).and_then(|ids| ids.first()) else {
        return;
    };
    let Some(base_health) = DungeonMobType::KingMidas.base_health() else { return };
    let remaining = base_health * (KING_MIDAS_HITS_TO_KILL - hit_count) as u64 / KING_MIDAS_HITS_TO_KILL as u64;
    let nametag_text = format!("\u{a7}c\u{a7}lKing Midas \u{a7}a{}\u{a7}c\u{2764}", format_health(remaining));

    let Some((nametag_entity, _)) = world.entities.get_mut(&nametag_id) else { return };
    nametag_entity.metadata.custom_name = Some(nametag_text);
    let metadata = nametag_entity.metadata.clone();
    for player in world.players.values_mut() {
        player.write_packet(&PacketEntityMetadata {
            entity_id: VarInt(nametag_id),
            metadata: metadata.clone(),
        });
    }
}

/// Removes the next golden armor piece (helmet -> chestplate -> leggings -> boots, for hits 1
/// through 4) from both the tracked `Equipment` and every connected player's live view of the
/// entity - `entity_equipment` alone only gets picked up on chunk (re)join, so a currently
/// visible hit needs its own direct `EntityEquipment` broadcast, same as `kill_mob` below does
/// for its death packets.
fn strip_next_king_midas_armor_piece(world: &mut World, entity_id: EntityId, hit_count: u8) {
    let slot = match hit_count {
        1 => 4, // helmet
        2 => 3, // chest
        3 => 2, // legs
        _ => 1, // boots
    };

    if let Some(equipment) = world.entity_equipment.get_mut(&entity_id) {
        match hit_count {
            1 => equipment.helmet = None,
            2 => equipment.chest = None,
            3 => equipment.legs = None,
            _ => equipment.boots = None,
        }
    }

    for player in world.players.values_mut() {
        player.write_packet(&EntityEquipment {
            entity_id: VarInt(entity_id),
            item_slot: slot,
            item_stack: None,
        });
    }
}

/// Plays the death animation + a species-appropriate death sound for `entity_id`, then
/// despawns it (which also cleans up equipment/nametag/AI state via `World::tick`'s removal
/// loop). Looks up the entity's variant itself so every caller gets the right sound "for
/// free" instead of each hardcoding one.
pub fn kill_mob(world: &mut World, entity_id: EntityId) {
    let Some((entity, _)) = world.entities.get(&entity_id) else { return };
    let pos = entity.position;
    let sounds = death_sounds_for(&entity.metadata.variant);

    for player in world.players.values_mut() {
        player.write_packet(&EntityStatus {
            entity_id,
            logic_op_code: ENTITY_DEATH_STATUS,
        });
        for death_sound in &sounds {
            player.write_packet(&SoundEffect {
                sound: death_sound.sound.id(),
                pos_x: pos.x,
                pos_y: pos.y,
                pos_z: pos.z,
                volume: death_sound.volume,
                pitch: death_sound.pitch,
            });
        }
    }

    let is_mimic = world.entity_mob_ai.get(&entity_id).is_some_and(|state| state.archetype == DungeonMobType::Mimic);
    if is_mimic {
        spawn_mimic_kill_decoy(world, pos);
        world.server_mut().dungeon.record_mimic_killed();
    }

    // Only credits the crypt this specific mob was spawned for (see `entity_crypt_room`,
    // populated by `Dungeon::superboom_at`) - blowing the crypt block alone no longer counts.
    if world.entity_crypt_room.remove(&entity_id).is_some() {
        world.server_mut().dungeon.record_crypt_killed();
    }

    world.despawn_entity(entity_id);

    // If this was a tracked starred mob, its room owes one less death before the map can show
    // a checkmark - redraw the map once the last one is gone so the change is visible right
    // away instead of waiting for some unrelated map update.
    if let Some(room_index) = world.entity_starred_mob_room.remove(&entity_id) {
        let dungeon = &mut world.server_mut().dungeon;
        if let Some(room) = dungeon.rooms.get_mut(room_index) {
            room.starred_mobs_remaining = room.starred_mobs_remaining.saturating_sub(1);
            if room.starred_mobs_remaining == 0 {
                dungeon.update_map_for_room(room_index);
                let granted_wither = dungeon.maybe_grant_door_key(room_index, Some(pos), crate::dungeon::room::secrets::PickupKind::Wither);
                let granted_blood = dungeon.maybe_grant_door_key(room_index, Some(pos), crate::dungeon::room::secrets::PickupKind::Blood);
                // Unconditional (not gated on a Wither/Blood door existing) - see its own doc
                // comment. `granted_wither || granted_blood` tells it whether a key room's own
                // companion TNT already dropped here, so it doesn't also drop a redundant
                // second one.
                dungeon.grant_room_clear_rewards(room_index, pos, granted_wither || granted_blood);
            }
        }
    }
}

/// Spawns and instantly "kills" a fully bare, invisible decoy baby zombie at `pos` - purely so
/// OdinClient's `onEntityDeath` mimic-kill heuristic still fires for the real mimic.
///
/// That heuristic (confirmed by tracing its actual bytecode, not assumed) requires the dying
/// `EntityZombie` to have completely empty equipment across `getEquipmentInSlot(0..=3)` (held
/// item + 3 of the 4 armor slots - the exact slot ordering wasn't fully verified). Since the
/// real mimic keeps its dyed leather armor and skull for visual accuracy to real Hypixel, it
/// can't satisfy that check itself. This decoy is a second, throwaway entity with zero
/// equipment in every slot: spawned, sent the same "death" `EntityStatus` the real mimic gets
/// (which is what makes a client fire `LivingDeathEvent` for an entity in the first place, per
/// `kill_mob` above), then despawned immediately after. It's invisible and never interactable,
/// so nothing about the visible mimic's death changes - this is purely a signal for that one
/// mod's detection, riding on the same status-packet mechanism `kill_mob` already uses for
/// every dungeon mob death.
///
/// The death `EntityStatus` packet is written into the *same chunk packet buffer* `spawn_entity`
/// just used for the spawn packet, not sent directly via `player.write_packet` - those are two
/// different delivery paths that flush at different points in the tick (chunk buffers get
/// copied into each player's outgoing buffer once per tick; `player.write_packet` goes out
/// immediately), so writing status directly could reach the client *before* the spawn packet
/// that makes it a known entity at all, in which case it's just silently ignored - a real bug
/// this had the first time, not a hypothetical one.
fn spawn_mimic_kill_decoy(world: &mut World, pos: DVec3) {
    let mut metadata = EntityMetadata::new(EntityVariant::Zombie {
        is_child: true,
        is_villager: false,
        is_converting: false,
        is_attacking: false,
    });
    metadata.is_invisible = true;

    let Ok(decoy_id) = world.spawn_entity(pos, metadata, NoEntityImpl) else { return };

    let chunk_x = (pos.x.floor() as i32) >> 4;
    let chunk_z = (pos.z.floor() as i32) >> 4;
    if let Some(chunk) = world.chunk_grid.get_chunk_mut(chunk_x, chunk_z) {
        chunk.packet_buffer.write_packet(&EntityStatus {
            entity_id: decoy_id,
            logic_op_code: ENTITY_DEATH_STATUS,
        });
    }

    world.despawn_entity(decoy_id);
}

/// One sound to play at a dungeon mob's death position - `kill_mob` plays every entry in the
/// `Vec` `death_sounds_for` returns, in order, all at that same position.
struct DeathSound {
    sound: Sounds,
    volume: f32,
    pitch: f32,
}

/// Per explicit correction: there is no zombie death sound anywhere in real dungeon mob deaths,
/// and the double "orb pickup" ding plays for EVERY dungeon mob death, minibosses included - not
/// just the mundane `EntityVariant::Zombie` grunts/soldiers/etc. `EntityVariant::Player` (Zombie
/// Commander, Crypt Dreadlord, King Midas, every other human-skinned miniboss - see `spawner.rs`'s
/// `MobBaseKind::Player`) additionally sounds like an actual player dying, and Skeleton/Enderman/
/// Blaze minibosses (Skeleton Master/Lord, Sniper, Fels, etc.) additionally keep their own real
/// vanilla death sound - both layered on top of the universal orb pair, never instead of it.
fn death_sounds_for(variant: &EntityVariant) -> Vec<DeathSound> {
    let mut sounds = match variant {
        EntityVariant::Skeleton { .. } => vec![DeathSound { sound: Sounds::SkeletonDeath, volume: 1.0, pitch: 1.0 }],
        EntityVariant::Enderman { .. } => vec![DeathSound { sound: Sounds::EndermenDeath, volume: 1.0, pitch: 1.0 }],
        EntityVariant::Blaze => vec![DeathSound { sound: Sounds::BlazeDeath, volume: 1.0, pitch: 1.0 }],
        EntityVariant::Player => vec![DeathSound { sound: Sounds::PlayerDeath, volume: 1.0, pitch: 1.0 }],
        _ => Vec::new(),
    };
    sounds.push(DeathSound { sound: Sounds::Orb, volume: 1.0, pitch: 1.492 });
    sounds.push(DeathSound { sound: Sounds::Orb, volume: 0.5, pitch: 1.73 });
    sounds
}

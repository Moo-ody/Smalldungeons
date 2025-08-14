use rand::prelude::IndexedRandom;
use std::collections::HashMap;

use crate::net::packets::client_bound::chat::Chat;
use crate::net::packets::client_bound::sound_effect::SoundEffect;
use crate::net::packets::packet::SendPacket;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::Player;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::particles::ParticleTypes;
use crate::server::utils::sounds::Sounds;
use crate::server::world::World;

/// register in World as: world.weirdos.insert(id, puzzle)
pub struct ThreeWeirdos {
    pub id: u32,
    pub origin: BlockPos,        // left chest position
    pub names: [String; 3],      // left, middle, right
    pub lines: [String; 3],      // statements shown in chat
    pub correct_index: usize,    // 0..=2 (which chest holds reward)
    pub solved_or_failed: bool,
    pub proximity_announced: bool,
    pub armor_ids: [i32; 3],     // spawned armorstands (for later polish)
}

impl ThreeWeirdos {
    pub fn spawn_at(world: &mut World, origin: BlockPos) -> u32 {
        // Unique-ish id
        let new_id = world.weirdos_next_id;
        world.weirdos_next_id += 1;

        // 3 unique names from the Hypixel-ish list
        const NAMES: &[&str] = &[
            "Ardis","Baxter","Benson","Carver","Elmo","Eveleth","Hope","Hugo","Lino","Luverne",
            "Madelia","Marshall","Melrose","Montgomery","Morris","Ramsey","Rose","Victoria",
            "Virginia","Willmar","Winona",
        ];
        let mut rng = rand::rng();
        let mut pool = NAMES.to_vec();
        pool.shuffle(&mut rng);
        let names = [pool[0].to_string(), pool[1].to_string(), pool[2].to_string()];

        // choose which chest is correct
        let correct = (0..3).choose(&mut rng).unwrap();

        // very simple “exactly one truth” script:
        // - the correct weirdo says the reward is in their chest (true)
        // - one liar says their chest has the reward (false)
        // - the other liar says "both others are lying" (false)
        let mut lines = ["".to_string(), "".to_string(), "".to_string()];
        for i in 0..3 {
            lines[i] = if i == correct {
                format!("My chest has the reward, and I'm telling the truth!")
            } else if i == (correct + 1) % 3 {
                format!("My chest has the reward!")
            } else {
                format!("Both of the others are lying!")
            };
        }

        // place 3 chests in a row (x+, z unchanged)
        for i in 0..3 {
            world.set_block_at(Blocks::Chest { direction: crate::server::block::block_parameter::Direction::North }, origin.x + (i as i32)*2, origin.y, origin.z);
        }

        // spawn 3 armor stands behind each chest (purely cosmetic for now)
        let mut armor_ids = [0; 3];
        for i in 0..3 {
            if let Ok(eid) = world.spawn_entity(
                DVec3::new(origin.x as f64 + 0.5 + (i as f64)*2.0, origin.y as f64 + 0.0, origin.z as f64 - 1.0),
                EntityMetadata::new(EntityVariant::ArmorStand),
                WeirdoNpc {},
            ) {
                armor_ids[i] = eid;
            }
        }

        // register clickable chests
        for i in 0..3 {
            let pos = BlockPos { x: origin.x + (i as i32)*2, y: origin.y, z: origin.z };
            world.interactable_blocks.insert(pos, crate::server::block::block_interact_action::BlockInteractAction::WeirdoGuess {
                puzzle_id: new_id,
                index: i,
            });
        }

        let puzzle = ThreeWeirdos {
            id: new_id,
            origin,
            names,
            lines,
            correct_index: correct,
            solved_or_failed: false,
            proximity_announced: false,
            armor_ids,
        };
        world.weirdos.insert(new_id, puzzle);
        new_id
    }

    /// Call from World::tick to show lines once when a player is close.
    pub fn tick(&mut self, world: &mut World) {
        if self.solved_or_failed || self.proximity_announced {
            return;
        }
        // announce when any player comes within ~6 blocks of middle chest
        let mid = DVec3::new(self.origin.x as f64 + 2.0, self.origin.y as f64 + 0.5, self.origin.z as f64 + 0.5);
        let someone_close = world.players.values().any(|p| p.position.distance_to(&mid) <= 6.0);
        if !someone_close { return; }

        self.proximity_announced = true;
        let header = format!("§e§lThree Weirdos§r — exactly one tells the truth.");
        for player in world.players.values() {
            let _ = player.send_packet(Chat { component: crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder::new(&header).build(), typ: 0 });
            for i in 0..3 {
                let label = match i { 0 => "Left", 1 => "Middle", _ => "Right" };
                let line = format!("§7{} {}§r: §f{}", label, self.names[i], self.lines[i]);
                let _ = player.send_packet(Chat { component: crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder::new(&line).build(), typ: 0 });
            }
            let hint = "§8Right-click a chest (left/middle/right) to choose.";
            let _ = player.send_packet(Chat { component: crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder::new(hint).build(), typ: 0 });
        }
    }

    pub fn handle_guess(&mut self, world: &mut World, who: &Player, index: usize) {
        if self.solved_or_failed { return; }
        self.solved_or_failed = true;

        let ok = index == self.correct_index;
        if ok {
            // success
            let msg = format!("§aPUZZLE SOLVED! §7{} §awasn't fooled by §e{}§a. Good job!",
                              who.profile.username, self.names[(index+1)%3]);
            for p in world.players.values() {
                let _ = p.send_packet(Chat { component: crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder::new(&msg).build(), typ: 0 });
                let _ = p.send_packet(SoundEffect { sounds: Sounds::Orb, volume: 1.0, pitch: 1.2, x: who.position.x, y: who.position.y, z: who.position.z });
            }
            // little particle pop at the chosen chest
            let chest = DVec3::new(self.origin.x as f64 + 0.5 + (index as f64)*2.0, self.origin.y as f64 + 1.0, self.origin.z as f64 + 0.5);
            world.spawn_entity(
                chest,
                EntityMetadata::new(EntityVariant::Bat { hanging: false }), // harmless temp particle carrier
                RewardPop { ticks: 6 }
            ).ok();
        } else {
            // fail
            let msg = format!("§cPUZZLE FAIL! §7{} §cwas fooled by §e{}§c. Yikes!", who.profile.username, self.names[self.correct_index]);
            for p in world.players.values() {
                let _ = p.send_packet(Chat { component: crate::server::utils::chat_component::chat_component_text::ChatComponentTextBuilder::new(&msg).build(), typ: 0 });
                let _ = p.send_packet(SoundEffect { sounds: Sounds::EndermanPortal, volume: 1.0, pitch: 0.5, x: who.position.x, y: who.position.y, z: who.position.z });
            }
        }

        // make chests no longer clickable
        for i in 0..3 {
            let pos = BlockPos { x: self.origin.x + (i as i32)*2, y: self.origin.y, z: self.origin.z };
            world.interactable_blocks.remove(&pos);
        }
    }
}

struct WeirdoNpc;
impl EntityImpl for WeirdoNpc {
    fn tick(&mut self, _e: &mut Entity) {}
}

struct RewardPop { ticks: u32 }
impl EntityImpl for RewardPop {
    fn tick(&mut self, e: &mut Entity) {
        if e.ticks_existed % 2 == 0 {
            for p in e.world_mut().players.values() {
                let pkt = crate::net::packets::client_bound::particles::Particles::new(
                    ParticleTypes::Cloud,
                    e.position,
                    DVec3::ZERO,
                    0.05,
                    6,
                    true,
                    None
                ).unwrap();
                let _ = p.send_packet(pkt);
            }
        }
        if e.ticks_existed >= self.ticks { e.world_mut().despawn_entity(e.id); }
    }
}

#[derive(Debug, Clone)]
pub struct ThreeWeirdos {
    pub id: u32,
    // world position for this instance (we’ll spawn near 20,69,20 later)
    pub origin: (i32, i32, i32),
}

impl ThreeWeirdos {
    pub fn new(id: u32, origin: (i32, i32, i32)) -> Self {
        Self { id, origin }
    }
}

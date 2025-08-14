use crate::net::packets::client_bound::block_change::BlockChange;
use crate::net::packets::client_bound::entity::destroy_entities::DestroyEntities;
use crate::net::packets::client_bound::spawn_mob::PacketSpawnMob;
use crate::net::packets::client_bound::spawn_object::PacketSpawnObject;
// use crate::net::packets::client_bound::position_look::PositionLook;
// use crate::net::packets::packet::SendPacket;
// use crate::net::packets::client_bound::particles::Particles;
// use crate::dungeon::puzzles::three_weirdos::ThreeWeirdos;
// use crate::net::packets::packet::SendPacket;
use crate::net::packets::packet_registry::ClientBoundPacket;
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_pos::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::chunk::chunk_grid::ChunkGrid;
use crate::server::entity::entity::{Entity, EntityId, EntityImpl};
use crate::server::entity::entity_metadata::EntityMetadata;
use crate::server::player::player::{ClientId, Player};
use crate::server::server::Server;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::player_list::PlayerList;
use std::collections::{HashMap, VecDeque};
#[derive(Debug, Clone, Copy)]
pub struct TacticalInsertionMarker {
    pub client_id: ClientId,
    pub return_tick: u64,
    pub origin: DVec3,
    pub damage_echo_window_ticks: u64,
    pub yaw: f32,
    pub pitch: f32,
    // absolute tick schedule for sounds to play before return
    // entries are (due_tick, sound, volume, pitch)
}

#[derive(Debug, Clone, Copy)]
pub struct ScheduledSound {
    pub due_tick: u64,
    pub sound: crate::server::utils::sounds::Sounds,
    pub volume: f32,
    pub pitch: f32,
}



pub struct World {
    /// Don't use directly!!, use .server_mut() instead
    /// This is unsafe,
    /// but since server should be alive for the entire program this is fine (I hope)
    pub server: *mut Server,

    pub chunk_grid: ChunkGrid,
    pub interactable_blocks: HashMap<BlockPos, BlockInteractAction>,

    pub player_info: PlayerList, // might need to be per player, not sure.

    // entity ids are always positive so they could theoretically be unsigned but minecraft uses signed ints in vanilla and casting might cause weird behavior, also assumes we ever reach the end of i32 though so it might be fine
    pub next_entity_id: i32,
    pub players: HashMap<ClientId, Player>,
    pub entities: HashMap<EntityId, (Entity, Box<dyn EntityImpl>)>,

    pub entities_for_removal: VecDeque<EntityId>,

        // add below `entities_for_removal`
    pub weirdos_next_id: u32,
    // Scheduled tactical insertions (teleport back after delay)
    pub tactical_insertions: Vec<(TacticalInsertionMarker, Vec<ScheduledSound>)>,
   // pub weirdos: HashMap<u32, crate::dungeon::puzzles::three_weirdos::ThreeWeirdos>,


    // pub commands: Vec<Command>
    
    // pub player_info: PlayerList,
    pub tick_count: u64,
}

impl World {

pub fn new() -> World {
    World {
        server: std::ptr::null_mut(),

        chunk_grid: ChunkGrid::new(14),
        interactable_blocks: HashMap::new(),

        player_info: PlayerList::new(),

        next_entity_id: 1,
        players: HashMap::new(),
        entities: HashMap::new(),
        entities_for_removal: VecDeque::new(),

        weirdos_next_id: 1,
//      weirdos: HashMap::new(),
        tactical_insertions: Vec::new(),
        tick_count: 0,
    }
}


    pub fn server_mut<'a>(&self) -> &'a mut Server {
        unsafe { self.server.as_mut().expect("server is null") }
    }

    pub fn new_entity_id(&mut self) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }

    pub fn spawn_entity<E : EntityImpl + 'static>(
        &mut self,
        position: DVec3,
        metadata: EntityMetadata,
        mut entity_impl: E,
    ) -> anyhow::Result<EntityId> {
        let world_ptr: *mut World = self;
        let mut entity = Entity::new(
            world_ptr,
            self.new_entity_id(),
            position,
            metadata,
        );
        if entity.metadata.variant.is_object() {
            for player in self.players.values() {
                player.send_packet(PacketSpawnObject::from_entity(&entity))?;
            }
        } else {
            for player in self.players.values() {
                player.send_packet(PacketSpawnMob::from_entity(&entity))?;
            }
        }

        entity_impl.spawn(&mut entity);
        
        let id = entity.id;
        self.entities.insert(id, (entity, Box::new(entity_impl)));
        Ok(id)
    }

    /// adds the entity id to 
    pub fn despawn_entity(&mut self, entity_id: EntityId) {
        self.entities_for_removal.push_back(entity_id)
    }

    pub fn tick(&mut self) -> anyhow::Result<()> {
        self.tick_count = self.tick_count.wrapping_add(1);
        let mut packets: Vec<ClientBoundPacket> = Vec::new();

        if !self.entities_for_removal.is_empty() {
            let mut packet = DestroyEntities {
                entity_ids: Vec::with_capacity(self.entities_for_removal.len()),
            };
            
            while let Some(entity_id) = self.entities_for_removal.pop_front() {
                if let Some((mut entity, mut entity_impl)) = self.entities.remove(&entity_id) {
                    entity_impl.despawn(&mut entity);
                }
                packet.entity_ids.push(entity_id)
            }
            
            packets.push(packet.into());
        }
        for (entity, entity_impl) in self.entities.values_mut() {
            entity.tick(entity_impl, &mut packets);
        }
        // Process scheduled tactical insertions
        self.process_tactical_insertions()?;

        for player in self.players.values_mut() {
            for packet in &packets {
                packet.clone().send_packet(player.client_id, &player.network_tx)?;
            }
        }
        Ok(())
    }

    fn process_tactical_insertions(&mut self) -> anyhow::Result<()> {
        if self.tactical_insertions.is_empty() { return Ok(()); }

        // Re-import for send_packet usage
        use crate::net::packets::client_bound::position_look::PositionLook;
        use crate::net::packets::packet::SendPacket;
        use crate::net::packets::client_bound::sound_effect::SoundEffect;

        // Drain due markers
        let now = self.tick_count;
        let mut remaining: Vec<(TacticalInsertionMarker, Vec<ScheduledSound>)> = Vec::with_capacity(self.tactical_insertions.len());

        for (mut marker, mut sounds) in self.tactical_insertions.drain(..) {
            // Emit any due sounds first
            if let Some(player) = self.players.get(&marker.client_id) {
                // Make sounds follow the player by using their current position
                let (x, y, z) = (player.position.x, player.position.y, player.position.z);
                let mut future: Vec<ScheduledSound> = Vec::new();
                for s in sounds.drain(..) {
                    if s.due_tick <= now {
                        let _ = SoundEffect { sounds: s.sound, volume: s.volume, pitch: s.pitch, x, y, z }
                            .send_packet(player.client_id, &player.network_tx);
                    } else {
                        future.push(s);
                    }
                }
                sounds = future;
            }

            // Handle return teleport once
            if marker.return_tick <= now {
                if let Some(player) = self.players.get(&marker.client_id) {
                    let _ = PositionLook {
                        x: marker.origin.x,
                        y: marker.origin.y,
                        z: marker.origin.z,
                        yaw: marker.yaw,
                        pitch: marker.pitch,
                        // 0 => absolute yaw/pitch so we face the original direction
                        flags: 0,
                    }.send_packet(player.client_id, &player.network_tx);
                }
                marker.return_tick = u64::MAX; // prevent repeat
            }

            // Keep scheduling if there are future sounds pending
            if !sounds.is_empty() {
                remaining.push((marker, sounds));
            }
        }

        self.tactical_insertions = remaining;
        Ok(())
    }

    pub fn set_block_at(&mut self, block: Blocks, x: i32, y: i32, z: i32) {
        let server = self.server_mut();
        for (client_id, _) in server.world.players.iter() {
            BlockChange {
                block_pos: BlockPos { x, y, z },
                block_state: block.get_block_state_id()
            };
            use crate::net::packets::packet::SendPacket;
            BlockChange { block_pos: BlockPos { x, y, z }, block_state: block.get_block_state_id() }
                .send_packet(*client_id, &server.network_tx)
                .unwrap();
        }
        self.chunk_grid.set_block_at(block, x, y, z);
    }

    pub fn get_block_at(&self, x: i32, y: i32, z: i32) -> Blocks {
        self.chunk_grid.get_block_at(x, y, z)
    }

    pub fn fill_blocks(&mut self, block: Blocks, start: BlockPos, end: BlockPos) {
        // should probably use a multi block change instead,
        // however it only applies to one chunk,
        // so you'd need to track the chunks and im not bothering with that
        iterate_blocks(start, end, |x, y, z| {
            self.set_block_at(block, x, y, z)
        })
    }
}

// from what ive read. this shouldn't have overhead and should inline the closure (hopefully)
/// iterates over the blocks in area between start and end
/// and runs a function
#[inline(always)]
pub fn iterate_blocks<F>(
    start: BlockPos,
    end: BlockPos,
    mut func: F,
)
where F : FnMut(i32, i32, i32)
{
    let x0 = start.x.min(end.x);
    let y0 = start.y.min(end.y);
    let z0 = start.z.min(end.z);

    let x1 = start.x.max(end.x);
    let y1 = start.y.max(end.y);
    let z1 = start.z.max(end.z);

    for x in x0..=x1 {
        for z in z0..=z1 {
            for y in y0..=y1 {
                func(x, y, z);
            }
        }
    }
}


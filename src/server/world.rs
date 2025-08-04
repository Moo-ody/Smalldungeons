use crate::net::packets::client_bound::block_change::BlockChange;
use crate::net::packets::client_bound::entity::destroy_entities::DestroyEntities;
use crate::net::packets::client_bound::spawn_mob::PacketSpawnMob;
use crate::net::packets::client_bound::spawn_object::PacketSpawnObject;
use crate::net::packets::packet::SendPacket;
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

    // pub commands: Vec<Command>
    
    // pub player_info: PlayerList,
}

impl World {

    pub fn new() -> World {
        World {
            server: std::ptr::null_mut(),

            chunk_grid: ChunkGrid::new(14),
            interactable_blocks: HashMap::new(),

            player_info: PlayerList::new(),

            next_entity_id: 1, // might have to start at 1
            players: HashMap::new(),
            entities: HashMap::new(),
            entities_for_removal: VecDeque::new(),

            // commands: Vec::new()
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
        for player in self.players.values_mut() {
            for packet in &packets {
                packet.clone().send_packet(player.client_id, &player.network_tx)?;
            }
        }
        Ok(())
    }

    pub fn set_block_at(&mut self, block: Blocks, x: i32, y: i32, z: i32) {
        let server = self.server_mut();
        for (client_id, _) in server.world.players.iter() {
            BlockChange {
                block_pos: BlockPos { x, y, z },
                block_state: block.get_block_state_id()
            }.send_packet(*client_id, &server.network_tx).unwrap();
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
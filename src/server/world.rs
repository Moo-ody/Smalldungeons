use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{DestroyEntites, SpawnMob, SpawnObject};
use crate::net::var_int::VarInt;
use crate::server::block::block_interact_action::BlockInteractAction;
use crate::server::block::block_position::BlockPos;
use crate::server::block::blocks::Blocks;
use crate::server::chunk::chunk_grid::ChunkGrid;
use crate::server::entity::entity::{Entity, EntityId, EntityImpl};
use crate::server::entity::entity_metadata::EntityMetadata;
use crate::server::player::player::{ClientId, Player};
use crate::server::server::Server;
use crate::server::utils::dvec3::DVec3;
use crate::server::utils::player_list::PlayerList;
use std::collections::{HashMap, VecDeque};

pub const VIEW_DISTANCE: u8 = 6;

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
    pub spawn_point: DVec3,
    pub spawn_yaw: f32,
    pub spawn_pitch: f32,
}

impl World {

    pub fn new() -> World {
        World {
            server: std::ptr::null_mut(),

            chunk_grid: ChunkGrid::new(16, 13, 13),
            interactable_blocks: HashMap::new(),

            player_info: PlayerList::new(),

            next_entity_id: 1, // might have to start at 1
            players: HashMap::new(),
            entities: HashMap::new(),
            entities_for_removal: VecDeque::new(),

            spawn_point: DVec3::ZERO,
            spawn_yaw: 0.0,
            spawn_pitch: 0.0,
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

    pub fn spawn_entity<E : EntityImpl + 'static>(&mut self,position: DVec3, metadata: EntityMetadata, mut entity_impl: E,) -> anyhow::Result<EntityId> {
        let world_ptr: *mut World = self;
        let mut entity = Entity::new(
            world_ptr,
            self.new_entity_id(),
            position,
            metadata,
        );

        const MOTION_CLAMP: f64 = 3.9;
        
        if entity.metadata.variant.is_object() {
            let packet = SpawnObject {
                entity_id: VarInt(entity.id),
                entity_variant: entity.metadata.variant.get_id(),
                x: (entity.position.x * 32.0).floor() as i32,
                y: (entity.position.y * 32.0).floor() as i32,
                z: (entity.position.z * 32.0).floor() as i32,
                yaw: (entity.yaw * 256.0 / 360.0) as i8,
                pitch: (entity.pitch * 256.0 / 360.0) as i8,
                data: 0,
                velocity_x: (entity.velocity.x.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
                velocity_y: (entity.velocity.y.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
                velocity_z: (entity.velocity.z.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
            };
            for player in self.players.values_mut() {
                player.write_packet(&packet);
            }
        } else {
            let packet = SpawnMob {
                entity_id: VarInt(entity.id),
                entity_variant: entity.metadata.variant.get_id(),
                x: (entity.position.x * 32.0).floor() as i32,
                y: (entity.position.y * 32.0).floor() as i32,
                z: (entity.position.z * 32.0).floor() as i32,
                yaw: (entity.yaw * 256.0 / 360.0) as i8,
                pitch: (entity.pitch * 256.0 / 360.0) as i8,
                head_pitch: (entity.yaw * 256.0 / 360.0) as i8, // head yaw for head pitch here is vanilla mappings. Maybe the mapping is wrong?
                velocity_x: (entity.velocity.x.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
                velocity_y: (entity.velocity.y.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
                velocity_z: (entity.velocity.z.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
                metadata: entity.metadata.clone(),
            };
            for player in self.players.values_mut() {
                player.write_packet(&packet);
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
        // should maybe write packets to chunk
        if !self.entities_for_removal.is_empty() {
            let mut packet = DestroyEntites {
                entities: Vec::with_capacity(self.entities_for_removal.len())
            };
            
            while let Some(entity_id) = self.entities_for_removal.pop_front() {
                if let Some((mut entity, mut entity_impl)) = self.entities.remove(&entity_id) {
                    entity_impl.despawn(&mut entity);
                }
                packet.entities.push(VarInt(entity_id))
            }
            
            for player in self.players.values_mut() {
                player.write_packet(&packet)
            }
        }

        let mut buf = PacketBuffer::new();

        for (entity, entity_impl) in self.entities.values_mut() {
            entity.tick(entity_impl, &mut buf);
        }
        for player in self.players.values_mut() {
            player.packet_buffer.copy_from(&buf);
            // player.flush_packets();
        }
        Ok(())
    }

    pub fn set_block_at(&mut self, block: Blocks, x: i32, y: i32, z: i32) {
        self.chunk_grid.set_block_at(block, x, y, z);
    }

    pub fn get_block_at(&self, x: i32, y: i32, z: i32) -> Blocks {
        self.chunk_grid.get_block_at(x, y, z)
    }

    pub fn set_spawn_point(&mut self, position: DVec3, yaw: f32, pitch: f32) {
        self.spawn_point = position;
        self.spawn_yaw = yaw;
        self.spawn_pitch = pitch;
    }

    pub fn fill_blocks(&mut self, block: Blocks, start: BlockPos, end: BlockPos) {
        iterate_blocks(start, end, |x, y, z| {
            self.set_block_at(block, x, y, z)
        })
    }
}

/// iterates over the blocks in area between start and end
/// and runs a function
#[inline(always)]
pub fn iterate_blocks<F>(
    start: BlockPos,
    end: BlockPos,
    mut callback: F,
) where 
    F : FnMut(i32, i32, i32)
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
                callback(x, y, z);
            }
        }
    }
}
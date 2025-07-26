use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::EntityTeleport;
use crate::server::entity::entity_metadata::EntityMetadata;
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;

pub type EntityId = i32;

/// provides functionality to an entity
pub trait EntityImpl {

    /// runs when an entity is spawned
    /// used to initialize more complex stuff
    fn spawn(&mut self, entity: &mut Entity) {}

    /// runs when an entity is ticked
    /// used to add custom functionality to an entity
    fn tick(&mut self, entity: &mut Entity);

    // todo: semi-despawn, i.e goes out of render distance
    
    /// runs when an entity is despawned
    /// used to clean up more complicated stuff
    fn despawn(&mut self, entity: &mut Entity) {}
}

/// represents an entity, its position, rotation, and its variant
pub struct Entity {
    world: *mut World,
    pub id: EntityId,

    pub position: DVec3,
    pub velocity: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,

    pub last_position: DVec3,
    pub last_yaw: f32,
    pub last_pitch: f32,
    
    pub ticks_existed: u32,
    
    pub metadata: EntityMetadata,
}

impl Entity {

    pub fn new(
        world: *mut World,
        id: EntityId,
        position: DVec3,
        metadata: EntityMetadata,
    ) -> Self {
        Self {
            world,
            id,
            position,
            velocity: DVec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            last_position: DVec3::ZERO,
            last_yaw: 0.0,
            last_pitch: 0.0,
            ticks_existed: 0,
            metadata,
        }
    }

    pub fn world_mut<'a>(&self) -> &'a mut World {
        unsafe { self.world.as_mut().unwrap() }
    }

    pub fn tick(
        &mut self,
        entity_impl: &mut Box<dyn EntityImpl>,
        packets: &mut PacketBuffer
    ) {
        // this might not be a good idea
        let test: *mut Entity = self;
        let entity = unsafe { test.as_mut().unwrap() };
        entity_impl.tick(self);
        
        if self.position != self.last_position {
            // TODO: if distance is < 8 blocks use entity rel move
            packets.write_packet(&EntityTeleport {
                entity_id: entity.id,
                pos_x: entity.position.x,
                pos_y: entity.position.y,
                pos_z: entity.position.z,
                yaw: entity.yaw,
                pitch: entity.pitch,
                on_ground: entity.on_ground,
            });
            self.last_position = self.position;
        }
        self.ticks_existed += 1;
    }
}


/// used for entities with no implementation
pub struct NoEntityImpl;

impl EntityImpl for NoEntityImpl {
    fn spawn(&mut self, entity: &mut Entity) {}
    fn tick(&mut self, entity: &mut Entity) {}
    fn despawn(&mut self, entity: &mut Entity) {}
}

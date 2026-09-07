use crate::net::packets::packet_buffer::PacketBuffer;
use crate::net::protocol::play::clientbound::{EntityTeleport, EntityYawRotate, SpawnMob, SpawnObject, SpawnPlayer};
use crate::net::protocol::play::serverbound::EntityInteractionType;
use crate::net::var_int::VarInt;
use crate::server::chunk::chunk::Chunk;
use crate::server::entity::entity_metadata::EntityMetadata;
use crate::server::player::player::{ClientId, Player};
use crate::server::utils::dvec3::DVec3;
use crate::server::world::World;
use uuid::Uuid;

pub type EntityId = i32;

/// provides functionality to an entity
pub trait EntityImpl {

    fn spawn(&mut self, _: &mut Entity, _: &mut PacketBuffer) {}

    fn despawn(&mut self, _: &mut Entity, _: &mut PacketBuffer) {}

    /// runs when an entity is ticked
    /// used to add custom functionality to an entity
    fn tick(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer);

    /// Returns `true` if this entity fully handled the interaction (e.g. Mort's dialogue, an
    /// armor stand terminal) - the caller uses this to decide whether the player's held item
    /// should also fire its right-click ability, so a plain mob with no special interaction
    /// still lets pearls/etherwarp/hyperion/etc. fire when clicked.
    fn interact(&mut self, _: &mut Entity, _: &mut Player, _: &EntityInteractionType) -> bool { false }

    /// Called when a player-fired projectile (currently only the Terminator - see
    /// `ai::projectile::MobProjectileImpl`'s `on_block_hit` doc comment for why mob-fired shots
    /// are excluded) hits this entity mid-flight. `velocity` is the projectile's own velocity at
    /// the moment of impact, not the shooter's current look angle - important for an arced shot,
    /// whose real flight direction has curved away from wherever the shooter is aiming *now*.
    /// Returns whether the projectile should stop here: `true` despawns it immediately (the Ice
    /// Path silverfish's own behavior, and the default - matches every implementor before this
    /// could pierce at all); `false` lets it keep flying and potentially hit more entities the
    /// same tick (the Higher or Lower puzzle's blazes - the Terminator should pierce through one
    /// to reach another standing behind it, per explicit request).
    fn on_projectile_hit(&mut self, _: &mut Entity, _velocity: DVec3, _shooter_id: ClientId) -> bool { true }

    /// Whether `ai::projectile::sweep_hit` should even consider this entity a possible target for
    /// a player-fired shot in the first place. `false` by default - most entities in this codebase
    /// are either decorative puzzle machinery nobody should be able to redirect by shooting it
    /// (Creeper Beams' visible Creeper prop, its invisible Guardian/Squid beam-trick pair) or a
    /// real mob with no `on_projectile_hit` behavior to trigger anyway. An earlier, broader
    /// "skip only invisible/dropped-item entities" heuristic here let a Terminator shot hit the
    /// Creeper Beams Creeper prop instead of the sea lantern behind it whenever the two lined up,
    /// silently eating shots meant for the lantern - this explicit opt-in can't regress the same
    /// way for some future visible entity that isn't a shootable target either.
    fn wants_projectile_hits(&self) -> bool { false }
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
    pub uuid: Option<Uuid>, // For Player entities

    /// Which chunk's `entities` list this entity is actually registered in - kept in sync
    /// with `position` by `World::tick()`'s chunk-migration pass. Without this, an entity
    /// that walks to a different chunk than the one it spawned in would still be listed in
    /// the *old* chunk forever (nothing else updates that list), leaving a stale ID that
    /// panics whenever something assumes chunk membership means the entity still exists.
    pub current_chunk: (i32, i32),
}

impl Entity {

    pub fn new(
        world: *mut World,
        id: EntityId,
        position: DVec3,
        metadata: EntityMetadata,
    ) -> Self {
        let current_chunk = ((position.x.floor() as i32) >> 4, (position.z.floor() as i32) >> 4);
        Self {
            world,
            id,
            position,
            velocity: DVec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            last_position: position,
            last_yaw: 0.0,
            last_pitch: 0.0,
            ticks_existed: 0,
            metadata,
            uuid: None,
            current_chunk,
        }
    }

    pub fn world_mut<'a>(&self) -> &'a mut World {
        unsafe { self.world.as_mut().unwrap() }
    }
    
    pub fn enter_view() {
        
    }
    
    pub fn write_spawn_packet(&self, buffer: &mut PacketBuffer) {
        let variant = &self.metadata.variant;
        if variant.is_player() {
            if let Some(uuid) = self.uuid {
                buffer.write_packet(&SpawnPlayer {
                    entity_id: VarInt(self.id),
                    uuid,
                    x: self.position.x,
                    y: self.position.y,
                    z: self.position.z,
                    yaw: self.yaw,
                    pitch: self.pitch,
                    current_item: 0,
                    metadata: self.metadata.clone(),
                });
            }
        } else if variant.is_object() {
            // Log SpawnObject creation
            buffer.write_packet(&SpawnObject {
                entity_id: VarInt(self.id),
                entity_variant: variant.get_id(),
                x: self.position.x,
                y: self.position.y,
                z: self.position.z,
                yaw: self.yaw,
                pitch: self.pitch,
                data: variant.object_data(),
                velocity_x: self.velocity.x,
                velocity_y: self.velocity.y,
                velocity_z: self.velocity.z,
            });
            
            // CRITICAL: For DroppedItem, we MUST send metadata packet immediately after SpawnObject
            // The item visual comes from metadata slot 10, not the SpawnObject data field
            use crate::net::protocol::play::clientbound::PacketEntityMetadata;
            buffer.write_packet(&PacketEntityMetadata {
                entity_id: VarInt(self.id),
                metadata: self.metadata.clone(),
            });
        } else {
            buffer.write_packet(&SpawnMob {
                entity_id: VarInt(self.id),
                entity_variant: variant.get_id(),
                x: self.position.x,
                y: self.position.y,
                z: self.position.z,
                yaw: self.yaw,
                pitch: self.pitch,
                head_yaw: self.yaw,
                velocity_x: self.velocity.x,
                velocity_y: self.velocity.y,
                velocity_z: self.velocity.z,
                metadata: self.metadata.clone(),
            });
        }
    } 
    
    pub fn tick(
        &mut self,
        entity_impl: &mut Box<dyn EntityImpl>,
        packet_buffer: &mut PacketBuffer
    ) {
        entity_impl.tick(self, packet_buffer);
        
        if self.position != self.last_position {
            packet_buffer.write_packet(&EntityTeleport {
                entity_id: self.id,
                pos_x: self.position.x,
                pos_y: self.position.y,
                pos_z: self.position.z,
                yaw: self.yaw,
                pitch: self.pitch,
                on_ground: self.on_ground,
            });
            // `EntityTeleport`'s yaw is the BODY orientation only - 1.8's mob models render the
            // head as a separate part driven by its own "head yaw", which vanilla keeps in sync
            // by also sending a dedicated Entity Head Look packet (`EntityYawRotate` here, same
            // 0x19 id) alongside any move/look update. Without this, a mob's body direction can
            // change but its head stays frozen at whatever it was on spawn - this was silently
            // never sent anywhere in this codebase until now (confirmed - the struct existed but
            // had zero call sites), which is exactly why turning to face a player never visibly
            // turned any mob's head.
            packet_buffer.write_packet(&EntityYawRotate {
                entity_id: VarInt(self.id),
                yaw: (self.yaw * 256.0 / 360.0) as i32 as i8,
            });
            self.last_position = self.position;
        }
        self.ticks_existed += 1;
    }
    
    pub fn chunk_position(&self) -> (i32, i32) {
        ((self.position.x.floor() as i32) >> 4, (self.position.z.floor() as i32) >> 4)
    }
    
    pub fn chunk_mut<'a>(&self) -> Option<&'a mut Chunk> {
        let (x, z) = self.chunk_position();
        self.world_mut().chunk_grid.get_chunk_mut(x, z)
    }
}


/// used for entities with no implementation
pub struct NoEntityImpl;

impl EntityImpl for NoEntityImpl {
    fn tick(&mut self, _: &mut Entity, _: &mut PacketBuffer) {}
}

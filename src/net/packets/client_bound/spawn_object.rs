use crate::net::packets::packet::{finish_packet, ClientBoundPacketImpl};
use crate::net::packets::packet_write::PacketWrite;
use crate::net::var_int::VarInt;
use crate::partial_packet;
use crate::server::entity::entity::{Entity, EntityId};
use crate::server::entity::entity_metadata::EntityVariant;
use crate::server::utils::dvec3::DVec3;
use tokio::io::{AsyncWrite, AsyncWriteExt};

const MOTION_CLAMP: f64 = 3.9;

#[derive(Debug, Clone)]
pub struct PacketSpawnObject {
    pub entity_id: i32,
    pub entity_variant: i8,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: i8,
    pub pitch: i8,
    pub data: i32,
    pub speed_x: i16,
    pub speed_y: i16,
    pub speed_z: i16,
}

impl PacketSpawnObject {
    pub fn new(
        entity_id: EntityId,
        entity_variant: EntityVariant,
        position: DVec3,
        velocity: DVec3,
        yaw: f32,
        pitch: f32,
        data: i32, // todo maybe in entity variant, allow representing it?
    ) -> Self {
        Self {
            entity_id,
            entity_variant: entity_variant.get_id(),
            x: (position.x * 32.0).floor() as i32,
            y: (position.y * 32.0).floor() as i32,
            z: (position.z * 32.0).floor() as i32,
            yaw: (yaw * 256.0 / 360.0) as i8,
            pitch: (pitch * 256.0 / 360.0) as i8,
            data,
            speed_x: (velocity.x.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
            speed_y: (velocity.y.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
            speed_z: (velocity.z.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
        }
    }

    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            entity_id: entity.id,
            entity_variant: entity.metadata.variant.get_id(),
            x: (entity.position.x * 32.0).floor() as i32,
            y: (entity.position.y * 32.0).floor() as i32,
            z: (entity.position.z * 32.0).floor() as i32,
            yaw: (entity.yaw * 256.0 / 360.0) as i8,
            pitch: (entity.pitch * 256.0 / 360.0) as i8,
            data: 0,
            speed_x: (entity.velocity.x.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
            speed_y: (entity.velocity.y.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
            speed_z: (entity.velocity.z.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
        }
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for PacketSpawnObject {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut payload = Vec::new();
        partial_packet!(payload =>
            VarInt(0x0e),
            VarInt(self.entity_id),
            self.entity_variant,
            self.x,
            self.y,
            self.z,
            self.pitch,
            self.yaw,
            self.data
        );
        if self.data > 0 {
            self.speed_x.write(&mut payload);
            self.speed_y.write(&mut payload);
            self.speed_z.write(&mut payload);
        }
        writer.write_all(&finish_packet(payload)).await
    }
}

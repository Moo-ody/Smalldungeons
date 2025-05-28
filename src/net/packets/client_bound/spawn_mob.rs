use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use crate::server::entity::entity_enum::EntityTrait;
use crate::server::entity::metadata::{Metadata, MetadataImpl};
use async_trait::async_trait;
use tokio::io::{AsyncWrite, AsyncWriteExt};

///
///
#[derive(Debug)]
pub struct SpawnMob {
    entity_id: i32,
    entity_type: i8,
    x: i32,
    y: i32,
    z: i32,
    yaw: i8,
    pitch: i8,
    head_pitch: i8,
    velocity_x: i16,
    velocity_y: i16,
    velocity_z: i16,
    metadata: Metadata
}

impl SpawnMob {
    pub fn from_entity<E: EntityTrait + MetadataImpl>(entity: &mut E) -> SpawnMob {
        let id = entity.get_id();
        let entity_base = entity.get_entity();
        let motion_clamp = 3.9;
        
        SpawnMob {
            entity_id: entity_base.entity_id as i32,
            entity_type: id,
            x: (entity_base.pos.x * 32.0).floor() as i32,
            y: (entity_base.pos.y * 32.0).floor() as i32,
            z: (entity_base.pos.z * 32.0).floor() as i32,
            yaw: (entity_base.yaw * 256.0 / 360.0) as i8,
            pitch: (entity_base.pitch * 256.0 / 360.0) as i8,
            head_pitch: (entity_base.head_yaw * 256.0 / 360.0) as i8, // head yaw for head pitch here is vanilla mappings. Maybe the mapping is wrong?
            velocity_x: (entity_base.motion.x.clamp(-motion_clamp, motion_clamp) * 8000.0) as i16,
            velocity_y: (entity_base.motion.y.clamp(-motion_clamp, motion_clamp) * 8000.0) as i16,
            velocity_z: (entity_base.motion.z.clamp(-motion_clamp, motion_clamp) * 8000.0) as i16,
            metadata: entity.create_meta_data()
        }
    }
}

#[async_trait]
impl ClientBoundPacketImpl for SpawnMob {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let buf = build_packet!(
            0x0F,
            VarInt(self.entity_id),
            self.entity_type,
            self.x,
            self.y,
            self.z,
            self.yaw,
            self.pitch,
            self.head_pitch,
            self.velocity_x,
            self.velocity_y,
            self.velocity_z,
            self.metadata
        );
        writer.write_all(&buf).await
    }
    
}
use crate::build_packet;
use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::varint::VarInt;
use crate::server::entity::entity::Entity;
use crate::server::entity::entity_type::EntityType;
use crate::server::entity::metadata::Metadata;
use anyhow::bail;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::io::{AsyncWrite, AsyncWriteExt};

const MOTION_CLAMP: f64 = 3.9;

#[derive(Debug, Clone)]
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
    metadata: HashMap<i8, Metadata>
}

impl SpawnMob {
    pub fn from_entity(entity: &Entity) -> anyhow::Result<Self> {
        if entity.entity_type == EntityType::Player {
            bail!("Player cannot be spawned as a mob.");
        }
        Ok(Self {
            entity_id: entity.entity_id,
            entity_type: entity.entity_type.get_id(),
            x: (entity.pos.x * 32.0).floor() as i32,
            y: (entity.pos.y * 32.0).floor() as i32,
            z: (entity.pos.z * 32.0).floor() as i32,
            yaw: (entity.yaw * 256.0 / 360.0) as i8,
            pitch: (entity.pitch * 256.0 / 360.0) as i8,
            head_pitch: (entity.head_yaw * 256.0 / 360.0) as i8, // head yaw for head pitch here is vanilla mappings. Maybe the mapping is wrong?
            velocity_x: (entity.motion.x.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
            velocity_y: (entity.motion.y.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
            velocity_z: (entity.motion.z.clamp(-MOTION_CLAMP, MOTION_CLAMP) * 8000.0) as i16,
            metadata: entity.metadata.clone()
        })
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
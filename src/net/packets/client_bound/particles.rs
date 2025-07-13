use crate::net::packets::packet::{finish_packet, ClientBoundPacketImpl};
use crate::net::packets::packet_write::PacketWrite;
use crate::net::var_int::{write_var_int, VarInt};
use crate::server::utils::particles::ParticleTypes;
use crate::server::utils::vec3d::DVec3;
use crate::partial_packet;
use anyhow::bail;
use std::io::{Error, ErrorKind};
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct Particles {
    particle_type: ParticleTypes,
    x: f32,
    y: f32,
    z: f32,
    offset_x: f32,
    offset_y: f32,
    offset_z: f32,
    speed: f32,
    count: i32,
    long_distance: bool,
    args: Option<Vec<i32>>,
}

impl Particles {
    pub fn new(typ: ParticleTypes, pos: DVec3, offset: DVec3, speed: f32, count: i32, long_distance: bool, args: Option<Vec<i32>>) -> anyhow::Result<Self> {
        let count_is_some = typ.get_arg_count().is_some();
        if count_is_some && args.is_none() || !count_is_some && args.is_some() {
            bail!("Invalid arguments for particle type: type: {:?}, args: {:?}", typ, args);
        }

        Ok(
            Self {
                particle_type: typ,
                x: pos.x as f32,
                y: pos.y as f32,
                z: pos.z as f32,
                offset_x: offset.x as f32,
                offset_y: offset.y as f32,
                offset_z: offset.z as f32,
                speed,
                count,
                long_distance,
                args,
            }
        )
    }
}

#[async_trait::async_trait]
impl ClientBoundPacketImpl for Particles {
    async fn write_to<W: AsyncWrite + Unpin + Send>(&self, writer: &mut W) -> std::io::Result<()> {
        let mut payload = Vec::new();

        partial_packet!(payload =>
            VarInt(0x2A),
            self.particle_type,
            self.long_distance,
            self.x,
            self.y,
            self.z,
            self.offset_x,
            self.offset_y,
            self.offset_z,
            self.speed,
            self.count,
        );

        if let Some(arg_count) = self.particle_type.get_arg_count() {
            if let Some(args) = self.args.as_ref() {
                for i in 0..arg_count {
                    write_var_int(&mut payload, *args.get(i as usize).ok_or_else(|| Error::new(ErrorKind::InvalidData, "Args index out of bounds. Did you use too many arguments?"))?);
                }
            }
        }

        writer.write_all(&finish_packet(payload)).await
    }
}
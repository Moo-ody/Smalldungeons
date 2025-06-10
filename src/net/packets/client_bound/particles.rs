use crate::net::packets::packet::ClientBoundPacketImpl;
use crate::net::packets::packet_write::PacketWrite;
use crate::net::var_int::write_var_int;
use crate::server::utils::particles::ParticleTypes;
use crate::server::utils::vec3f::Vec3f;
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
    pub fn new(typ: ParticleTypes, pos: Vec3f, offset: Vec3f, speed: f32, count: i32, long_distance: bool, args: Option<Vec<i32>>) -> anyhow::Result<Self> {
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
        write_var_int(&mut payload, 0x2A);
        self.particle_type.write(&mut payload);
        self.long_distance.write(&mut payload);
        self.x.write(&mut payload);
        self.y.write(&mut payload);
        self.z.write(&mut payload);
        self.offset_x.write(&mut payload);
        self.offset_y.write(&mut payload);
        self.offset_z.write(&mut payload);
        self.speed.write(&mut payload);
        self.count.write(&mut payload);
        if let Some(arg_count) = self.particle_type.get_arg_count() {
            if let Some(args) = self.args.as_ref() {
                for i in 0..arg_count {
                    write_var_int(&mut payload, *args.get(i as usize).ok_or_else(|| Error::new(ErrorKind::InvalidData, "Args index out of bounds. Did you use too many arguments?"))?);
                }
            }
        }

        let mut buf = Vec::new();
        write_var_int(&mut buf, payload.len() as i32);
        buf.extend_from_slice(&payload);

        writer.write_all(&buf).await?;
        Ok(())
    }
}
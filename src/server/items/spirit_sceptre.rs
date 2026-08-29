use crate::net::protocol::play::clientbound::EntityTeleport;
use crate::net::packets::packet_buffer::PacketBuffer;
use crate::server::block::block_collision::check_block_collisions;
use crate::server::entity::entity::{Entity, EntityImpl};
use crate::server::entity::entity_metadata::{EntityMetadata, EntityVariant};
use crate::server::player::player::{ClientId, Player};
use crate::server::utils::aabb::AABB;
use crate::server::utils::dvec3::DVec3;

const MAX_LIFETIME_TICKS: u32 = 40; // ~2 seconds; tweak later
const SPEED_PER_TICK: f64 = 1.0; // blocks per tick (20 bps)

pub fn on_right_click(player: &mut Player) -> anyhow::Result<()> {
    let mut spawn_pos = player.player_eye_position();
    spawn_pos.y -= 1.0;
    let shooter = player.client_id;

    player.world_mut().spawn_entity(
        spawn_pos,
        EntityMetadata::new(EntityVariant::SpiritSceptreBat { hanging: false }),
        SpiritSceptreBatImpl {
            shooter_client_id: shooter,
        },
    )?;

    Ok(())
}

pub struct SpiritSceptreBatImpl {
    shooter_client_id: ClientId,
}

impl EntityImpl for SpiritSceptreBatImpl {
    fn tick(&mut self, entity: &mut Entity, packet_buffer: &mut PacketBuffer) {
        if entity.ticks_existed > MAX_LIFETIME_TICKS {
            entity.world_mut().despawn_entity(entity.id);
            return;
        }

        // Follow the shooter's aim.
        let world = entity.world_mut();
        let Some(shooter) = world.players.get(&self.shooter_client_id) else {
            world.despawn_entity(entity.id);
            return;
        };

        let dir: DVec3 = shooter.rotation_vec().normalize();
        entity.position = DVec3::new(
            entity.position.x + dir.x * SPEED_PER_TICK,
            entity.position.y + dir.y * SPEED_PER_TICK,
            entity.position.z + dir.z * SPEED_PER_TICK,
        );
        entity.yaw = shooter.yaw;
        entity.pitch = shooter.pitch;

        // Sync movement to all players (bat is a mob, so teleport is simplest).
        for _player in world.players.values() {
            let _ = packet_buffer.write_packet(&EntityTeleport {
                entity_id: entity.id,
                pos_x: entity.position.x,
                pos_y: entity.position.y,
                pos_z: entity.position.z,
                yaw: entity.yaw,
                pitch: entity.pitch,
                on_ground: false,
            });
        }

        // Collision check (approx bat projectile size).
        let aabb = AABB::from_height_width(0.5, 0.9).offset(entity.position);
        if check_block_collisions(world, &aabb) {
            crate::server::items::hyperion::handle_hyperion_explosion(world.server_mut(), entity.position);
            world.despawn_entity(entity.id);
        }
    }
}


use crate::net::network_message::NetworkMessage;
use crate::net::packets::client_bound::entity::entity_head_look::EntityHeadLook;
use crate::net::packets::client_bound::entity::entity_look::EntityLook;
use crate::net::packets::client_bound::entity::entity_look_move::EntityLookMove;
use crate::net::packets::client_bound::entity::entity_rel_move::EntityRelMove;
use crate::net::packets::client_bound::entity::entity_teleport::EntityTeleport;
use crate::net::packets::client_bound::spawn_mob::SpawnMob;
use crate::net::packets::packet::SendPacket;
use crate::net::packets::packet_registry::ClientBoundPacket;
use crate::server::entity::ai::ai_tasks::AiTasks;
use crate::server::entity::attributes::Attributes;
use crate::server::entity::entity_type::EntityType;
use crate::server::entity::look_helper::LookHelper;
use crate::server::entity::metadata::{BaseMetadata, Metadata};
use crate::server::player::{ClientId, Player};
use crate::server::utils::aabb::AABB;
use crate::server::utils::vec3f::Vec3f;
use crate::server::world::World;
use std::cmp::{max, min};
use std::collections::HashSet;
use std::mem::take;
use tokio::sync::mpsc::UnboundedSender;

/// type alias for entity ids.
///
/// alias for i32 since minecraft's ints are signed.
/// if we were to use u32 and went above the positive limit for i32's it would send negative id packets which may lead to undefined behavior
pub type EntityId = i32;

#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_id: EntityId,
    pub entity_type: EntityType,
    // pub entity_type_data: EntityTypeData,
    pub pos: Vec3f,
    pub motion: Vec3f,
    pub prev_pos: Vec3f,
    pub last_sent_pos: Vec3f,
    pub last_sent_yaw: f32,
    pub last_sent_pitch: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub aabb: AABB,
    pub height: f32,
    pub width: f32,
    pub ticks_existed: u32,
    pub health: f32,

    pub metadata: Metadata,

    pub attributes: Attributes,

    pub ai_tasks: Option<AiTasks>,
    pub ai_target: Option<EntityId>,

    pub look_helper: LookHelper,

    pub observing_players: HashSet<ClientId>
}

impl Entity {
    pub fn create_at(entity_type: EntityType, pos: Vec3f, id: EntityId) -> Entity {
        Entity {
            entity_id: id,
            entity_type,
            pos,
            motion: Vec3f::new_empty(),
            prev_pos: pos,
            last_sent_pos: pos,
            last_sent_yaw: 0.0,
            last_sent_pitch: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            head_yaw: 0.0,
            aabb: AABB::new_empty(), // aabb should be determined by height and width, which are determined by entity type and certain entity properties like size.
            health: 20.0, // todo: replace by using max health attribute, add requirement for attributes. could also make max health a normal param instead since its required but well see how i want to implement that in the attribute packet.
            height: 0.0,
            width: 0.0,
            ticks_existed: 0,

            metadata: Metadata {
                base: BaseMetadata {
                    name: stringify!(entity_type).to_owned()
                },
                entity: entity_type.metadata(),
            },

            attributes: Attributes::new(),

            ai_tasks: entity_type.get_tasks(),
            ai_target: None,

            look_helper: LookHelper::from_pos(pos, 10.0, 10.0),

            observing_players: HashSet::new()
        }
    }
    pub fn update_position(&mut self, x: f64, y: f64, z: f64) {
        self.prev_pos = self.pos;
        self.pos.x = x;
        self.pos.y = y;
        self.pos.z = z;
    }

    pub fn is_alive(&self) -> bool {
        !self.health.is_nan() && self.health > 0.0
    }

    pub fn update(mut self, world: &mut World, network_tx: &UnboundedSender<NetworkMessage>) -> Self {
        // i dont know where in vanilla this happens but its necessary for vanilla to handle the packet properly and it isnt in the packet handling section.
        // living update mods yaw/pitch stuff if it got an update but that doesnt happen via at least the watchclosest ai and it wouldnt even work for this.
        self.head_yaw = LookHelper::wrap_to_180(self.head_yaw);
        let mut current = self.update_state(world);

        // this has not been checked to see if its in the right order, its just here because it needs to be here for now.
        // world should probably have its own network tx clone so we dont need to pass it through here maybe? not sure.
        if let Some(packet) = current.get_pos_packet() {
            for player in current.observing_players.iter() {
                packet.clone().send_packet(*player, network_tx).unwrap_or_else(|err| println!("error updating entity position for client {player}: {err:?}"));
                EntityHeadLook::from_entity(&current).send_packet(*player, network_tx).unwrap_or_else(|err| println!("error updating entity head yaw for client {player}: {err:?}"));
            }
        }

        current.last_sent_pos = current.pos;
        current.last_sent_yaw = current.yaw;
        current.last_sent_pitch = current.pitch;

        current
    }

    pub fn load_for_player(&mut self, player: &Player, network_tx: &UnboundedSender<NetworkMessage>) -> anyhow::Result<()> {
        if self.entity_id == player.entity_id { return Ok(()); }
        self.observing_players.insert(player.client_id);

        // this should be replaced via a spawn entity function that gets the correct type of spawn entity packet, whether it be spawnmob, spawnplayer, spawnobject, etc.
        SpawnMob::from_entity(self)?.send_packet(player.client_id, network_tx)
    }

    pub fn update_state(mut self, world: &mut World) -> Self {
        // check despawn
        // sensing cache clear

        // target ai update

        if let Some(mut tasks) = take(&mut self.ai_tasks) {
            tasks.update(&mut self, world).unwrap_or_else(|err| println!("error updating entity ai tasks: {:?}", err));
            self.ai_tasks = Some(tasks);
        }
        // path navigation update

        // generic task update?

        // move helper update
        LookHelper::on_update_look(&mut self);
        // jump helper update
        self
    }

    pub fn get_pos_packet(&self) -> Option<ClientBoundPacket> {
        let rotated = self.last_sent_pitch != self.pitch || self.last_sent_yaw != self.yaw;
        // we may need resync logic if an entity moves more than like 8 blocks in a tick but that seems unlikely
        Some(if self.ticks_existed % 200 == 0 {
            ClientBoundPacket::from(EntityTeleport::from_entity(self))
        } else if self.pos != self.last_sent_pos {
            if rotated {
                ClientBoundPacket::from(EntityLookMove::from_entity(self))
            } else {
                ClientBoundPacket::from(EntityRelMove::from_entity(self))
            }
        } else if rotated {
            ClientBoundPacket::from(EntityLook::from_entity(self))
        } else { return None })
    }

    pub fn is_in_aabb_i32(
        &self,
        min_x: i32, min_y: i32, min_z: i32,
        max_x: i32, max_y: i32, max_z: i32,
    ) -> bool {
        // pmo
        let (min_x, min_y, min_z) = (min_x as f64, min_y as f64, min_z as f64);
        let (max_x, max_y, max_z) = (max_x as f64, max_y as f64, max_z as f64);
        let (x, y, z) = (self.pos.x, self.pos.y, self.pos.z);
        x >= min_x && x <= max_x &&
        y >= min_y && y <= max_y &&
        z >= min_z && z <= max_z
    }

    pub fn is_in_box_i32(
        &self,
        x: i32, y: i32, z: i32,
        width: i32, height: i32, length: i32,
    ) -> bool {
        // im too lazy to make this not repeat bunch of stuff
        let min_x = min(x, x + width);
        let min_y = min(y, y + height);
        let min_z = min(z, z + length);
        let max_x = max(x, x + width);
        let max_y = max(y, y + height);
        let max_z = max(z, z + length);
        self.is_in_aabb_i32(min_x, min_y, min_z, max_x, max_y, max_z)
    }
}